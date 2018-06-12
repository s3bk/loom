use std::collections::HashMap;
use std::rc::Rc;
use std::ops::Deref;
use document::{Node, NodeP, NodeListP};
use io::{Io};
use hyphenation::Hyphenator;
use commands::Command;
use indexmap::IndexMap;
use layout::{Atom, Glue, Writer};
use wheel::{Directory};
use istring::IString;

/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.

/// The Environment needs to be split in layout-relevant parts where
/// the Definition wins, and scope relevant parts where the Pattern wins

/// used at graph creation and possibly layout
pub struct LocalEnv {
    paths:          Vec<Directory>,
    commands:       HashMap<IString, Command>,
    targets:        HashMap<IString, NodeP>,
    groups:         IndexMap<(IString, IString), NodeP>,
    hyphenator:     Option<Hyphenator>,
    symbols:        IndexMap<IString, IString>
}

pub struct Fields {
    pub args:   Option<NodeListP>,
    pub body:   Option<NodeListP>
}
impl Fields {
    pub fn childs(&self, out: &mut Vec<NodeP>) {
        if let Some(ref args) = self.args {
            out.push(args.clone().into());
        }
        if let Some(ref body) = self.body {
            out.push(body.clone().into());
        }
    }
}

impl LocalEnv {
    pub fn new() -> LocalEnv {
        LocalEnv {
            paths:      vec![],
            commands:   HashMap::new(),
            targets:    HashMap::new(),
            groups:     IndexMap::new(),
            hyphenator: None,
            symbols:    IndexMap::new()
        }
    }
    pub fn add_command(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.into(), cmd);
    }
    pub fn add_path(&mut self, dir: Directory) {
        self.paths.push(dir);
    }
    pub fn add_target(&mut self, name: IString, target: NodeP) {
        self.targets.insert(name, target);
    }
    pub fn add_group(&mut self, opening: IString, closing: IString, node: NodeP) {
        self.groups.insert((opening, closing), node);
    }
    pub fn childs(&self, out: &mut Vec<NodeP>) {
        for n in self.targets.values() {
            out.push(n.clone());
        }
    }
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        self.targets.get(name)
    }
    pub fn targets<'a>(&'a self) -> impl Iterator<Item=(&'a IString, &'a NodeP)> {
        self.targets.iter()
    }
    pub fn set_hyphenator(&mut self, hyphenator: Hyphenator) {
        self.hyphenator = Some(hyphenator);
    }
    pub fn add_symbol(&mut self, src: &str, dst: &str) {
        self.symbols.insert(src.into(), dst.into());
    }
}

pub struct GraphLink {
    parent: Option<GraphChain>,
    local:  LocalEnv
}

#[derive(Clone)]
pub struct GraphChain {
    inner: Rc<GraphLink>
}

impl GraphChain {
    pub fn root(local: LocalEnv) -> GraphChain {
        GraphChain {
            inner: Rc::new(GraphLink {
                parent: None,
                local:  local
            })
        }
    }
    
    pub fn link(&self, local: LocalEnv) -> GraphChain {
        GraphChain {
            inner: Rc::new(GraphLink {
                parent: Some(self.clone()),
                local:  local
            })
        }
    }
    pub fn take(self) -> LocalEnv {
        match Rc::try_unwrap(self.inner) {
            Ok(e) => e.local,
            Err(_) => panic!("refcount > 1")
        }
    }
    
    fn find<F, T: ?Sized>(&self, cond: F) -> Option<&T> where
    F: Fn(&LocalEnv) -> Option<&T>
    {
        if let Some(v) = cond(&self.inner.local) {
            Some(v)
        } else if let Some(ref parent) = self.inner.parent {
            parent.find(cond)
        } else {
            None
        }
    }
    
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        self.find(|env| env.targets.get(name))
    }
    
    pub fn get_group(&self, q: &(IString, IString)) -> Option<&NodeP> {
        self.find(|env| env.groups.get(q))
    }

    pub fn get_command(&self, name: &str) -> Option<&Command> {
        self.find(|env| env.commands.get(name))
    }
    pub fn get_symbol(&self, name: &str) -> Option<&str> {
        match self.find(|env| env.symbols.get(name)) {
            Some(ref s) => Some(s.as_str()),
            None => None
        }
    }
}

impl Deref for GraphChain {
    type Target = LocalEnv;
    fn deref(&self) -> &LocalEnv {
        &self.inner.local
    }
}

#[derive(Clone)]
pub struct LayoutChain<'a> {
    parent: Option<&'a LayoutChain<'a>>,
    local:  &'a LocalEnv,
    fields: Option<&'a Fields>
}
impl<'a> Deref for LayoutChain<'a> {
    type Target = LocalEnv;
    fn deref(&self) -> &LocalEnv {
        self.local
    }
}
impl<'a> LayoutChain<'a> {
    pub fn root(env: &LocalEnv) -> LayoutChain {
        LayoutChain {
            parent: None,
            local:  env,
            fields: None
        }
    }

    pub fn link<'b: 'a, N: Node>(&'b self, node: &'b N) -> LayoutChain<'b> {
        match node.env() {
            Some(local) => {
                LayoutChain {
                    parent: Some(self),
                    local:  local,
                    fields: node.fields().or(self.fields)
                }
            }
            None => self.clone()
        }
    }
    
    pub fn with_fields<'b>(self, fields: Option<&'b Fields>) -> LayoutChain<'b>
    where 'a: 'b
    {
        LayoutChain {
            parent: self.parent,
            local:  self.local,
            fields: fields
        }
    }

    fn find<'b, F, T>(&'b self, cond: F) -> Option<&'b T> where
    F: Fn(&'b LayoutChain<'a>) -> Option<&'b T>
    {
        if let Some(v) = cond(self) {
            Some(v)
        } else if let Some(parent) = self.parent {
            parent.find(cond)
        } else {
            None
        }
    }
    
    pub fn fields(&self) -> Option<&Fields> {
        self.find(|c| c.fields)
    }
    
    pub fn hyphenator(&self) -> Option<&Hyphenator> {
        self.find(|c| c.hyphenator.as_ref())
    }
    pub fn hyphenate(&self, w: &mut Writer, word: Atom) {
        if let Some(hyphenator) = self.hyphenator() {
            if let Some(points) = hyphenator.get(word.text) {
                w.branch(&mut |b| {
                    b.add(&mut |w: &mut Writer| w.word(word) );
                        
                    for p in points.iter() {
                        let (left, right) = p.apply(word.text);
                        b.add(&mut |w: &mut Writer| {
                            w.word(Atom {
                                left:   word.left,
                                right:  Glue::None,
                                text:   left
                            });
                            w.punctuation(Atom {
                                left:   Glue::None,
                                right:  Glue::newline(),
                                text:   "-"
                            });
                            w.word(Atom {
                                left:   Glue::newline(),
                                right:  word.right,
                                text:   right
                            });
                        });
                    }
                });
                return;
            }
        }
        
        // fallback
        w.word(word);
    }
}

pub fn prepare_graph(_io: &Io) -> GraphChain {
    use commands;
    
    let mut e = LocalEnv::new();
    
    commands::register(&mut e);
    
    GraphChain::root(e)
}
