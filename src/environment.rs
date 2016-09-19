use std::collections::HashMap;
use std::sync::Arc;
use std::rc::Rc;
use std::borrow::BorrowMut;
use typeset::{TypeEngine, Font, UnscaledFont};
use layout::{TokenStream};
use blocks::BlockHandler;
use document;


/// The Environment can only be changed within the BlockHandler::process call
/// Is is therefore allowed to cache results whithin methods that do not involve
/// such calls.
pub struct Environment<'a> { 
    //commands:   HashMap<String, ...>
    blocks:         HashMap<String, Box<BlockHandler>>,
    tokens:         HashMap<String, TokenStream>,
    //words:          HashMap<String, Word>,
    
    /// loaded fonts (without specified size)
    /// use with .get_font()
    fonts:          HashMap<String, Box<UnscaledFont>>,
    
    /// default font
    default_font:   Option<Rc<Font>>,
    
    font_engines:   Vec<Box<TypeEngine>>,
    
    parent:         Option<&'a Environment<'a>>
}

impl<'a> Environment<'a> {
    pub fn process_block(&self, block: &document::Block, s: &mut TokenStream) -> bool {
        if let Some(b) = self.blocks.get(&block.name) {
            b.process(self, block, s);
            true
        } else {
            false
        }
    }
    pub fn add_block(&mut self, name: &str, b: Box<BlockHandler>) {
        self.blocks.insert(name.to_owned(), b);
    }
    pub fn use_token(&self, name: &str, s: &mut TokenStream) -> bool {
        if let Some(ts) = self.tokens.get(name) {
            s.extend(ts);
            true
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
    
    pub fn get_font(&self, name: &str, size: f32) -> Option<Rc<Font>> {
        self.fonts.get(name).map(|f| f.scale(size))
    }
    
    pub fn set_default_font(&mut self, f: Rc<Font>) {
        self.default_font = Some(f);
    }
    pub fn default_font(&self) -> Option<Rc<Font>> {
        self.default_font.clone()
    }
    
    pub fn add_font_engine(&mut self, e: Box<TypeEngine>) {
        self.font_engines.push(e)
    }
    
    pub fn extend(&self) -> Environment {
        Environment {
            blocks:         HashMap::new(),
            tokens:         HashMap::new(),
            //words:          HashMap::new(),
            fonts:          HashMap::new(),
            default_font:   self.default_font.clone(),
            font_engines:   vec![],
            parent:         Some(self)
        }
    }
    
    pub fn new() -> Environment<'static> {
        Environment {
            blocks:         HashMap::new(),
            tokens:         HashMap::new(),
            //words:          HashMap::new(),
            fonts:          HashMap::new(),
            default_font:   None,
            font_engines:   vec![],
            parent:         None
        }
    }
}

pub fn prepare_environment(e: &mut Environment) {
    use blocks::{Chapter, Term};
    use layout::Flex;
    use typeset::RustTypeEngine;
    use std::fmt::Debug;
    
    e.add_font_engine(RustTypeEngine::new());
    e.set_default_font(RustTypeEngine::default().scale(20.0));
    
    e.add_block("chapter", Box::new(Chapter::new()) as Box<BlockHandler>);
    e.add_block("term", Box::new(Term::new()) as Box<BlockHandler>);
    
    #[derive(Debug)]
    struct HFill {}
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
