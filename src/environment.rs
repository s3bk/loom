use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use typeset::{TypeEngine, Font, UnscaledFont};
use layout::{TokenStream};
use document::{Macro, NodeP};
use hyphenation::Hyphenator;
use document;
use parser;
use slog::{self, Logger};

type Command = Fn(&mut Environment, &[String]) -> bool;
type Handler = Fn(&mut Environment, &parser::Block) -> document::NodeP;


/// The Environment can only be changed within the Block::parse call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.
pub struct Environment<'a> {
    commands:       HashMap<String, Box<Command>>,
    blocks:         HashMap<String, Box<Handler>>,
    tokens:         HashMap<String, TokenStream>,
    hyphenator:     Option<Hyphenator>,
    
    /// loaded fonts (without specified size)
    /// use with .get_font()
    fonts:          HashMap<String, Box<UnscaledFont>>,
    
    /// default font
    default_font:   Option<Arc<Font>>,
    
    font_engines:   Vec<Box<TypeEngine>>,
    
    parent:         Option<&'a Environment<'a>>,
    
    paths:          Vec<PathBuf>,
    active_macro:   Option<&'a Macro>,
    targets:        HashMap<String, NodeP>,
    logger:         Logger
}

impl<'a> Environment<'a> {
    pub fn get_handler(&self, name: &str) -> Option<&Box<Handler>> {
        match self.blocks.get(name) {
            Some(c) => Some(c),
            None => match self.parent {
                Some(p) => p.get_handler(name),
                None => None
            }
        }
    }
    
    pub fn add_command(&mut self, name: &str, cmd: Box<Command>) {
        self.commands.insert(name.to_owned(), cmd);
    }
    pub fn get_command(&self, name: &str) -> Option<&Box<Command>> {
        match self.commands.get(name) {
            Some(c) => Some(c),
            None => match self.parent {
                Some(p) => p.get_command(name),
                None => None
            }
        }
    }
    pub fn add_handler(&mut self, name: &str, b: Box<Handler>) {
        self.blocks.insert(name.to_owned(), b);
    }
    pub fn use_token(&self, s: &mut TokenStream, name: &str) -> bool {
        if let Some(ts) = self.tokens.get(name) {
            s.extend(ts);
            true
        } else if let Some(p) = self.parent {
            p.use_token(s, name)
        } else {
            false
        }
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
    
    pub fn get_font(&self, name: &str, size: f32) -> Option<Arc<Font>> {
        if let Some(f) = self.fonts.get(name) {
            Some(f.scale(size))
        } else if let Some(p) = self.parent {
            p.get_font(name, size)
        } else {
            None
        }
    }
    
    pub fn set_default_font(&mut self, f: Arc<Font>) {
        self.default_font = Some(f);
    }
    pub fn default_font(&self) -> Option<Arc<Font>> {
        self.default_font.clone()
    }
    
    pub fn add_font_engine(&mut self, e: Box<TypeEngine>) {
        self.font_engines.push(e)
    }
    
    pub fn set_hyphenator(&mut self, hyphenator: Hyphenator) {
        self.hyphenator = Some(hyphenator);
        println!("hyphenator set");
    }
    pub fn hyphenator(&self) -> Option<&Hyphenator> {
        match self.hyphenator {
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
                        println!("{}-{}", left, right);
                        let mut s_branch = TokenStream::new();
                        s_branch.word(font.measure(&format!("{}-", left)))
                        .newline()
                        .word(font.measure(right));
                        s_branch
                    })
                );
                return;
            } else {
                println!("word not found: {}", word);
            }
        } else {
            println!("no hyphenator found");
        }
        
        s.word(font.measure(word));
    }
    
    fn add_path(&mut self, path: PathBuf) {
        self.paths.push(path);
    }
    
    fn search_file(&self, filename: &str) -> Option<PathBuf> {
        for dir in self.paths.iter() {
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
    
    pub fn set_macro(&mut self, m: &'a Macro) {
        self.active_macro = Some(m);
    }
    pub fn get_macro(&self) -> Option<&Macro> {
        self.active_macro
    }
    
    pub fn add_target(&mut self, name: String, target: NodeP) {
        self.targets.insert(name, target);
    }
    pub fn get_target(&self, name: &str) -> Option<&NodeP> {
        match self.targets.get(name) {
            Some(t) => Some(t),
            None => match self.parent {
                Some(p) => p.get_target(name),
                None => None
            }
        }
    }
    pub fn logger(&self, values: Vec<slog::OwnedKeyValue>) -> Logger {
        self.logger.new(values)
    }
    pub fn log(&self, record: &slog::Record) {
        self.logger.log(record)
    }
    
    pub fn new(logger: Logger) -> Environment<'static> {
        Environment {
            blocks:         HashMap::new(),
            tokens:         HashMap::new(),
            //words:          HashMap::new(),
            fonts:          HashMap::new(),
            default_font:   None,
            font_engines:   vec![],
            hyphenator:     None,
            parent:         None,
            commands:       HashMap::new(),
            targets:        HashMap::new(),
            paths:          vec![],
            active_macro:   None,
            logger:         logger
        }
    }
    
    pub fn extend(&self, values: Vec<slog::OwnedKeyValue>) -> Environment {
        Environment {
            default_font:   self.default_font.clone(),
            parent:         Some(self),
            ..              Environment::new(self.logger(values))
        }
    }
}   

pub fn prepare_environment(e: &mut Environment) {
    use layout::Flex;
    use typeset::RustTypeEngine;
    use std::env::var_os;
    use std::path::Path;
    
    let data_path: PathBuf = match var_os("LOOM_DATA") {
        Some(v) => v.into(),
        None => {
            let p = Path::new(file!()).parent().unwrap().join("doc");
            info!(e, "LOOM_DATA not set. Using {:?} instead.", p);
            p
        }
    };
    
    e.add_path(data_path);
    
    e.add_font_engine(RustTypeEngine::new());
    e.set_default_font(RustTypeEngine::default().scale(20.0));
    
    e.add_command("hyphens", Box::new(cmd_hyphens));
    
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
}

fn cmd_hyphens(env: &mut Environment, args: &[String]) -> bool {
    if args.len() != 1 {
        println!("expectec one argument");
        return false;
    }
    let ref filename = args[0];
    match env.search_file(&filename) {
        None => {
            println!("hyphens file not found: {}", &filename as &str);
            false
        },
        Some(path) => {
            let h = Hyphenator::load(&path);
            env.set_hyphenator(h);
            true
        }
    }
}
