use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use document::{Node, NodeP, P, NodeListP};
use io::IoRef;
use hyphenation::Hyphenator;
use document;
use parser;
use commands::Command;
use inlinable_string::InlinableString;
use ordermap::OrderMap;
use output::Output;
use layout::{Atom, Glue, Writer, Flex, StreamVec};

/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.

/// The Environment needs to be split in layout-relevant parts where
/// the Definition wins, and scope relevant parts where the Pattern wins

/// used at graph creation and possibly layout
pub struct LocalEnv {
    paths:          Vec<PathBuf>,
    commands:       HashMap<String, Command>,
    targets:        HashMap<String, NodeP>,
    groups:         OrderMap<(InlinableString, InlinableString), NodeP>,
    hyphenator:     Option<Hyphenator>
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
            hyphenator: None
        }
    }
    pub fn add_command(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_owned(), cmd);
    }
    fn add_path(&mut self, path: PathBuf) {
        self.paths.push(path);
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
                w.branch(word.left, word.right, points.len() + 1, &mut |b| {
                    b.add(&mut |w: &mut Writer| w.word(word) );
                        
                    for p in points.iter() {
                        let (left, right) = p.apply(word.text);
                        b.add(&mut |w: &mut Writer| {
                            w.word(Atom {
                                left:   word.left,
                                right:  Glue::newline(),
                                text:   &format!("{}-", left)
                            });
                            w.word(Atom {
                                left:   Glue::newline(),
                                right:  Glue::space(),
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
    
    
    pub fn search_file(&self, filename: &str) -> Option<PathBuf> {
        for dir in self.local.paths.iter() {
            let path = dir.join(filename);
            if path.is_file() {
                return Some(path);
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
}   

pub fn prepare_graph() -> LocalEnv {
    use layout::Flex;
    use commands;
    use std::env::var_os;
    use std::path::Path;
    
    let data_path: PathBuf = match var_os("LOOM_DATA") {
        Some(v) => v.into(),
        None => {
            let p = Path::new("data").into();
            println!("LOOM_DATA not set. Using '{:?}' instead.", p);
            p
        }
    };
    
    let mut e = LocalEnv::new();
    
    e.add_path(data_path);
    commands::register(&mut e);
    
    e
}
