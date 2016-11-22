use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use layout::{TokenStream};
use document::{Node, NodeP, P, NodeListP};
use io::IoRef;
use hyphenation::Hyphenator;
use document;
use parser;
use commands::Command;
use inlinable_string::InlinableString;
use ordermap::OrderMap;
use output::{Output, Writer, Word};

/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.

/// The Environment needs to be split in layout-relevant parts where
/// the Definition wins, and scope relevant parts where the Pattern wins

pub struct LayoutEnv<O: Output> {
    /// fields used only during layout()
    
    hyphenator:     Option<Hyphenator>,
    
    /// loaded fonts (without specified size)
    /// use with .get_font()
    //fonts:          HashMap<String, Box<O::UnscaledFont>>,
    
    /// default font
    default_font:   Option<Arc<O::Font>>,
    
    tokens:         HashMap<String, TokenStream<O>>
}

/// used at graph creation and possibly layout
pub struct LocalEnv {
    paths:          Vec<PathBuf>,
    commands:       HashMap<String, Command>,
    targets:        HashMap<String, NodeP>,
    groups:         OrderMap<(InlinableString, InlinableString), NodeP>
}
impl LocalEnv {
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

impl LayoutEnv {
    pub fn new() -> LayoutEnv<O> {
        LayoutEnv {
            hyphenator:     None,
            default_font:   None,
            //tokens:         HashMap::new()
        }
    }
    /*
    pub fn add_token(&mut self, name: &str, t: TokenStream<O>) {
        self.tokens.insert(name.to_owned(), t);
    }
    */
    /*
    pub fn load_font(&mut self, path: &str, name: &str) -> bool {
        {
            self.font_engines.iter_mut()
            .map(|e| e.load_font(path).ok())
            .filter_map(|o| o)
            .next()
        }
        .map(|f| self.fonts.insert(name.to_owned(), f))
        .is_some()
    }
    */
    pub fn set_default_font(&mut self, f: O::Font) {
        self.default_font = Some(f);
    }
    pub fn set_hyphenator(&mut self, hyphenator: Hyphenator) {
        self.hyphenator = Some(hyphenator);
        println!("hyphenator set");
    }
}
impl LocalEnv {
    fn new() -> LocalEnv {
        LocalEnv {
            paths:      vec![],
            commands:   HashMap::new(),
            targets:    HashMap::new(),
            groups:     OrderMap::new()
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
}

#[derive(Copy, Clone)]
pub struct GraphChain<'a> {
    parent:         Option<&'a GraphChain<'a>>,
    fields:         Option<&'a FieldLink<'a>>,
    local:          &'a LocalEnv,
}

#[derive(Copy, Clone)]
pub struct LayoutChain<'a> {
    parent:         Option<&'a LayoutChain<'a>>,
    fields:         Option<&'a FieldLink<'a>>,
    local:          &'a LocalEnv,
    layout:         &'a LayoutEnv,
    output:         &'a Output
}

impl<'a> GraphChain<'a> {
    pub fn root(locals: &'a LocalEnv) -> GraphChain<'a> {
        GraphChain {
            parent: None,
            locals: locals
        }
    }
    
    pub fn link(&'a self, locals: &'a LocalEnv) -> GraphChain<'a> {
        GraphChain {
            parent: Some(self),
            locals: locals
        }
    }
}

impl<'a, O> LayoutChain<'a, O> where O: Output {
    pub fn root(locals: &'a LocalEnv, layout: &'a LayoutEnv<O>, output: &'a O)
    -> LayoutChain<'a, O> {
        LayoutChain {
            parent: None,
            fields: None,
            locals: locals,
            layout: layout,
            output: output
        }
    }
    
    pub fn link(&'a self, locals: &'a LocalEnv) -> LayoutChain<'a, O> {
        LayoutChain {
            parent: Some(self),
            locals: locals,
            ..      self
        }
    }

    pub fn link_fields(&'a self, fields: &'a Fields) -> FieldLink<'a> {
        FieldLink {
            local:  fields,
            parent: self.fields
        }
    }
    
    pub fn with_fields(self, fields: Option<&'a FieldLink<'a>>) -> LayoutChain<'a, O> {
        LayoutChain {
            fields: fields,
            ..      self
        }
    }
    
    pub fn parent(&'a self) -> Option<&LayoutChain<'a, O>> {
        self.parent
    }
    
    pub fn fields(&'a self) -> Option<&FieldLink<'a>> {
        self.fields
    }
    
    pub fn get_command(&self, name: &str) -> Option<&Command> {
        match self.locals.commands.get(name) {
            Some(c) => Some(c),
            None => match self.parent {
                Some(p) => p.get_command(name),
                None => None
            }
        }
    }
    /*
    pub fn get_token(&self, name: &str) -> Option<&TokenStream> {
        match self.locals.layout.get(name) {
            Some(c) => Some(c),
            None => match self.parent {
                Some(p) => p.get_token(name),
                None => None
            }
        }
    }
    
    pub fn get_font(&self, name: &str, size: f32) -> Option<O::Font> {
        if let Some(f) = self.layout.fonts.get(name) {
            Some(f.scale(size))
        } else if let Some(p) = self.parent {
            p.get_font(name, size)
        } else {
            None
        }
    }
    */
    pub fn default_font(&self) -> Option<O::Font> {
        match self.locals.default_font {
            Some(ref f) => Some(f.clone()),
            None => match self.parent {
                Some(p) => p.default_font(),
                None => None
            }
        }
    }
    
    //resolve!(hyphenator -> self.locals.hyphenator )
    
    pub fn hyphenator(&self) -> Option<&Hyphenator> {
        match self.locals.hyphenator {
            Some(ref h) => Some(h),
            None => match self.parent {
                Some(p) => p.hyphenator(),
                None => None
            }
        }
    }
    pub fn hyphenate(&self, w: &mut Writer<O>, font: &O::Font, word: Word) {
        if let Some(hyphenator) = self.hyphenator() {
            if let Some(points) = hyphenator.get(word.text) {
                w.push(word.left, word.right, |l, o| {
                    l.word(o.measure(font, word.text));
                    
                    for p in points.iter() {
                        let (left, right) = p.apply(word.text);
                        
                        l.word(o.measure(font, &format!("{}-", left)));
                        l.newline(false);
                        l.word(o.measure(font, right));
                    }
                });
                
                return;
            }
        }
        
        // fallback
        w.push_word(w.output.measure(font, word));
    }
    
    
    pub fn search_file(&self, filename: &str) -> Option<PathBuf> {
        for dir in self.locals.paths.iter() {
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
        match self.locals.targets.get(name) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_target(name),
                None => None
            }
        }
    }
    
    pub fn get_group(&self, q: &(InlinableString, InlinableString)) -> Option<&NodeP> {
        match self.locals.groups.get(q) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_group(q),
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
