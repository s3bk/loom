use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use typeset::{TypeEngine, Font, UnscaledFont};
use layout::{TokenStream};
use document::{NodeP, P, NodeListP};
use io::IoRef;
use hyphenation::Hyphenator;
use document;
use parser;

type Command = fn(IoRef, Environment, &mut LocalEnv, &[String]) -> bool;

/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Args,
    Body
}

#[derive(Derivative)]
#[derivative(Debug, Default="new")]
pub struct LocalEnv {
    #[derivative(Debug="ignore")]
    hyphenator:     Option<Hyphenator>,
    
    /// loaded fonts (without specified size)
    /// use with .get_font()
    #[derivative(Debug="ignore")]
    fonts:          HashMap<String, Box<UnscaledFont>>,
    
    /// default font
    #[derivative(Debug="ignore")]
    default_font:   Option<Arc<Font>>,
    
    #[derivative(Debug="ignore")]
    font_engines:   Vec<Box<TypeEngine>>,
    
    paths:          Vec<PathBuf>,
    
    #[derivative(Debug="ignore")]
    commands:       HashMap<String, Command>,
    tokens:         HashMap<String, TokenStream>,
    targets:        HashMap<String, NodeP>,
    
    fields:         HashMap<Field, NodeListP>,
}

#[derive(Debug, Copy, Clone)]
pub struct Environment<'a> {
    parent:         Option<&'a Environment<'a>>,
    locals:         &'a LocalEnv
}

impl LocalEnv {
    pub fn add_command(&mut self, name: &str, cmd: Command) {
        self.commands.insert(name.to_owned(), cmd);
    }
    pub fn add_token(&mut self, name: &str, t: TokenStream) {
        self.tokens.insert(name.to_owned(), t);
    }
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
    pub fn set_default_font(&mut self, f: Arc<Font>) {
        self.default_font = Some(f);
    }
    pub fn add_font_engine(&mut self, e: Box<TypeEngine>) {
        self.font_engines.push(e)
    }
    pub fn set_hyphenator(&mut self, hyphenator: Hyphenator) {
        self.hyphenator = Some(hyphenator);
        println!("hyphenator set");
    }
    fn add_path(&mut self, path: PathBuf) {
        self.paths.push(path);
    }
    pub fn add_target(&mut self, name: &str, target: NodeP) {
        println!("add_target({}, …)", name);
        self.targets.insert(name.to_owned(), target);
    }
    pub fn set_field(&mut self, f: Field, n: NodeListP) {
        self.fields.insert(f, n);
    }
    /*
    pub fn add_group(&mut self, opening: &str, closing: &str, node: NodeP) {
        self.groups.insert((opening.to_owned(), closing.to_owned()), node)
    }
    */
    pub fn childs(&self, out: &mut Vec<NodeP>) {
        for n in self.targets.values() {
            out.push(n.clone());
        }
        for n in self.fields.values() {
            out.push(n.clone().into());
        }
    }
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        self.targets.get(name)
    }
    pub fn targets<'a>(&'a self) -> impl Iterator<Item=(&'a String, &'a NodeP)> {
        self.targets.iter()
    }
}
impl<'a> Environment<'a> {
    pub fn root(locals: &'a LocalEnv) -> Environment<'a> {
        Environment {
            parent: None,
            locals: locals
        }
    }
    
    pub fn link(&'a self, locals: &'a LocalEnv) -> Environment<'a> {
        Environment {
            parent:         Some(self),
            locals:         locals
        }
    }
    
    pub fn parent(&'a self) -> Option<&Environment<'a>> {
        self.parent
    }
    
    pub fn get_field(&self, f: Field) -> Option<NodeListP> {
        match self.locals.fields.get(&f) {
            Some(c) => Some(c.clone()),
            None => match self.parent {
                Some(p) => p.get_field(f),
                None => None
            }
        }
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
    pub fn get_token(&self, name: &str) -> Option<&TokenStream> {
        match self.locals.tokens.get(name) {
            Some(c) => Some(c),
            None => match self.parent {
                Some(p) => p.get_token(name),
                None => None
            }
        }
    }
    
    pub fn get_font(&self, name: &str, size: f32) -> Option<Arc<Font>> {
        if let Some(f) = self.locals.fonts.get(name) {
            Some(f.scale(size))
        } else if let Some(p) = self.parent {
            p.get_font(name, size)
        } else {
            None
        }
    }
    
    pub fn default_font(&self) -> Option<Arc<Font>> {
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
    pub fn hyphenate(&self, s: &mut TokenStream, word: &str, font: &Arc<Font>) {
        if let Some(hyphenator) = self.hyphenator() {
            if let Some(points) = hyphenator.get(word) {
                let mut s_default = TokenStream::new();
                s_default.word(font.measure(word));
                
                s.branch_many(s_default,
                    points.iter()
                    .map(|hyphen| hyphen.apply(word))
                    .map(|(left, right)| {
                        let mut s_branch = TokenStream::new();
                        s_branch.word(font.measure(&format!("{}-", left)))
                        .newline()
                        .word(font.measure(right));
                        s_branch
                    })
                );
                return;
            }
        }
        
        s.word(font.measure(word));
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
}   

pub fn prepare_environment() -> LocalEnv {
    use layout::Flex;
    use typeset::RustTypeEngine;
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
    
    e.add_font_engine(RustTypeEngine::new());
    e.set_default_font(RustTypeEngine::default().scale(20.0));
    
    commands::register(&mut e);
    
    #[derive(Debug)]
    struct HFill {}
    #[allow(unused_variables)]
    impl Flex for HFill {    
        fn stretch(&self, line_width: f32) -> f32 { line_width }
        fn shrink(&self, line_width: f32) -> f32 { 0.0 }
        fn width(&self, line_width: f32) -> f32 { line_width * 0.5 }
        fn height(&self, line_width: f32) -> f32 { 0.0 }
    }
    {
        let mut hfill = TokenStream::new();
        hfill.nbspace(Arc::new(HFill{}));
        e.add_token("hfill", hfill);
    }
    
    e
}

