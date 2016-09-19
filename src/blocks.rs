use std::cell::Cell;
use std::sync::Arc;
use std::ops;
use environment::Environment;
use document::{self, Node, Element};
use layout::{TokenStream, Flex};
use typeset::Font;

macro_rules! token {
    (
        $stream:ident, $font:ident, $space:ident, $a:expr
    ) => (
        $stream.word($font.measure($a));
    );
    
    (
        $stream:ident, $font:ident, $space:ident, $a:expr, _, $($b:tt)*
    ) => (
        $stream.word($font.measure($a));
        $stream.space($space.clone());
        token!($stream, $font, $space, $($b)*);
    );
    (
        $stream:ident, $font:ident, $space:ident, $a:expr, ~, $($b:tt)*
    ) => (
        $stream.word($font.measure($a));
        $stream.nbspace($space.clone());
        token!($stream, $font, $space, $($b)*);
    );
    (
        $stream:ident, $font:ident, $space:ident, $a:expr, $($b:tt)*
    ) => (
        $stream.word($font.measure($a));
        token!($stream, $font, $space, $($b)*);
    );
}

pub trait BlockHandler {
    fn process(&self, env: &Environment, b: &document::Block, s: &mut TokenStream);
    
    fn processChilds(&self, env: &Environment, b: &document::Block, s: &mut TokenStream) {
        for c in b.childs.iter() {
            self.processNode(env, c, s);
        }
    }
    
    fn processNode(&self, env: &Environment, node: &document::Node, s: &mut TokenStream) {
        match node {
            &Node::Block(ref b) => {
                if env.process_block(b, s) {
                } else {
                    let ref font = env.default_font().unwrap();
                    let space = font.space().flex(2.0);
                    token!(s, font, space,
                    "Unknown", ~, "Block", _, "'", &b.name, "'");
                    env.use_token("hfill", s);
                    s.newline();          
                }
            }
            &Node::Leaf(ref elements) => {
                self.processElements(env, elements, s);
                env.use_token("hfill", s);
                s.newline();
            },
            &Node::List(ref items) => self.processList(env, items, s)
        }
    }
    
    fn processElements(&self, env: &Environment, elements: &[Element], s: &mut TokenStream) {
        let ref font = env.default_font().unwrap();
        let space = font.space().flex(2.0);
        
        let mut append_space = false;
        for element in elements.iter() {
            match element {
                &Element::Word(ref e) |
                &Element::Reference(ref e) |
                &Element::Symbol(ref e) => {
                    if append_space {
                        s.space(space.clone());
                    }
                    s.word(font.measure(&e));
                    append_space = true;
                }
                &Element::Punctuation(ref e) => {
                    s.word(font.measure(&e));
                }
            }
        }
    }
    
    fn processList(&self, env: &Environment, items: &[Node], s: &mut TokenStream) {
        for item in items.iter() {
            let ref font = env.default_font().unwrap();
            s.word(font.measure("· "));
            self.processNode(env, item, s);
        }
    }
}

pub struct Chapter {
    counter:    Cell<u32>
}

fn roman_numerals(i: u32) -> String {
    use roman;
    roman::to(i as i32).unwrap_or_else(|| format!("{}", i))
}

fn increment_cell(cell: &Cell<u32>) -> u32 {
    let value = cell.get() + 1;
    cell.set(value);
    value
}

impl BlockHandler for Chapter {
    fn process(&self, env: &Environment, b: &document::Block, s: &mut TokenStream) {
        let counter = increment_cell(&self.counter);
        let ref title = b.argument;
        
        let ref font = env.default_font().unwrap();
        let space = font.space().flex(2.0);
        
        token!(s, font, space, "Chapter", ~, &roman_numerals(counter), ":");
        env.use_token("hfill", s);
        
        self.processElements(env, title, s);
        s.newline();
        
        self.processChilds(env, b, s);
    }
}
impl Chapter {
    pub fn new() -> Chapter {
        Chapter { counter: Cell::new(0) }
    }
}

pub struct Term {
    //reg:    RefCell<HashMap<String, 
    counter: Cell<u32>
}
impl Term {
    pub fn new() -> Term {
        Term {
            counter: Cell::new(0)
        }
    }
}
impl BlockHandler for Term {
    fn process(&self, env: &Environment, b: &document::Block, s: &mut TokenStream) {
        let ref title = b.argument;
        
        let ref font = env.default_font().unwrap();
        let space = font.space().flex(2.0);
        
        token!(s, font, space, "Term", ~, &format!("{}", increment_cell(&self.counter)));
        env.use_token("hfill", s);
        
        self.processElements(env, title, s);
        s.newline();
        
        self.processChilds(env, b, s);
    }
}
    
