use std::collections::HashMap;
use document::{NodeP, NodeListP};
use io::{IoRef, Directory, File, Entry};
use hyphenation::Hyphenator;
use commands::Command;
use inlinable_string::InlinableString;
use ordermap::OrderMap;
use layout::{Atom, Glue, Writer};

/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.

/// The Environment needs to be split in layout-relevant parts where
/// the Definition wins, and scope relevant parts where the Pattern wins

/// used at graph creation and possibly layout
pub struct LocalEnv {
    paths:          Vec<Directory>,
    commands:       HashMap<String, Command>,
    targets:        HashMap<String, NodeP>,
    groups:         OrderMap<(InlinableString, InlinableString), NodeP>,
    hyphenator:     Option<Hyphenator>,
    symbols:        OrderMap<InlinableString, InlinableString>
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

pub struct FieldLink<'a> {
    local:  &'a Fields,
    parent: Option<&'a FieldLink<'a>>
}
impl<'a> FieldLink<'a> {
    pub fn parent(&self) -> Option<&'a FieldLink<'a>> {
        self.parent
    }
    pub fn args(&self) -> Option<NodeListP> {
        self.local.args.clone()
    }
    pub fn body(&self) -> Option<NodeListP> {
        self.local.body.clone()
    }
}

impl LocalEnv {
    pub fn new() -> LocalEnv {
        LocalEnv {
            paths:      vec![],
            commands:   HashMap::new(),
            targets:    HashMap::new(),
            groups:     OrderMap::new(),
            hyphenator: None,
            symbols:    OrderMap::new()
        }
    }
    pub fn add_command(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_owned(), cmd);
    }
    fn add_path(&mut self, dir: Directory) {
        self.paths.push(dir);
    }
    pub fn add_target(&mut self, name: &str, target: NodeP) {
        println!("add_target({}, …)", name);
        self.targets.insert(name.to_owned(), target);
    }
    pub fn add_group(&mut self, opening: &str, closing: &str, node: NodeP) {
        self.groups.insert((opening.into(), closing.into()), node);
    }
    pub fn childs(&self, out: &mut Vec<NodeP>) {
        for n in self.targets.values() {
            out.push(n.clone());
        }
    }
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        self.targets.get(name)
    }
    pub fn targets<'a>(&'a self) -> impl Iterator<Item=(&'a String, &'a NodeP)> {
        self.targets.iter()
    }
    pub fn set_hyphenator(&mut self, hyphenator: Hyphenator) {
        self.hyphenator = Some(hyphenator);
    }
    pub fn add_symbol(&mut self, src: &str, dst: &str) {
        self.symbols.insert(src.into(), dst.into());
    }
}

#[derive(Copy, Clone)]
pub struct GraphChain<'a> {
    parent:         Option<&'a GraphChain<'a>>,
    fields:         Option<&'a FieldLink<'a>>,
    local:          &'a LocalEnv,
}

impl<'a> GraphChain<'a> {
    pub fn root(locals: &'a LocalEnv) -> GraphChain<'a> {
        GraphChain {
            parent: None,
            local:  locals,
            fields: None
        }
    }
    
    pub fn link(&'a self, locals: &'a LocalEnv) -> GraphChain<'a> {
        GraphChain {
            parent: Some(self),
            local:  locals,
            fields: self.fields
        }
    }
    
    pub fn link_fields(&'a self, fields: &'a Fields) -> FieldLink<'a> {
        FieldLink {
            local:  fields,
            parent: self.fields
        }
    }
    
    pub fn with_fields(self, fields: Option<&'a FieldLink<'a>>) -> GraphChain<'a> {
        GraphChain {
            fields: fields,
            ..      self
        }
    }
    
    pub fn fields(&'a self) -> Option<&FieldLink<'a>> {
        self.fields
    }
    
    pub fn hyphenator(&self) -> Option<&Hyphenator> {
        match self.local.hyphenator {
            Some(ref h) => Some(h),
            None => match self.parent {
                Some(p) => p.hyphenator(),
                None => None
            }
        }
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
    
    
    pub fn search_file(&self, filename: &str) -> Option<File> {
        for dir in self.local.paths.iter() {
            match dir.get(filename) {
                Ok(Entry::File(f)) => {
                    return Some(f);
                }
                _ => {}
            }
        }
        
        match self.parent {
            Some(p) => p.search_file(filename),
            None => None
        }
    }
    
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        match self.local.targets.get(name) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_target(name),
                None => None
            }
        }
    }
    
    pub fn get_group(&self, q: &(InlinableString, InlinableString)) -> Option<&NodeP> {
        match self.local.groups.get(q) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_group(q),
                None => None
            }
        }
    }

    pub fn get_command(&self, name: &str) -> Option<&Command> {
        match self.local.commands.get(name) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_command(name),
                None => None
            }
        }
    }
    pub fn get_symbol(&self, name: &str) -> Option<&str> {
        match self.local.symbols.get(name) {
            Some(t) => Some(&t),
            None => match self.parent {
                Some(p) => p.get_symbol(name),
                None => None
            }
        }
    }
}   

pub fn prepare_graph(io: IoRef) -> LocalEnv {
    use commands;
    
    let data_path = io.platform().data_dir();
    let mut e = LocalEnv::new();
    
    e.add_path(data_path);
    commands::register(&mut e);
    
    e
}
