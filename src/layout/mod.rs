use std::fmt::{self, Debug};
use std::rc::Rc;
use units::*;
use output::Output;

// private mods
mod glue;
mod paragraph;
mod generic_writer;
mod flex;
mod style;
pub mod columns;

pub use self::glue::Glue;
pub use self::paragraph::ParagraphLayout;
pub use self::generic_writer::{GenericWriter};
pub use self::flex::FlexMeasure;
pub use self::style::Style;
pub use self::columns::ColumnLayout;

// to flex or not to flex?
#[allow(unused_variables)]
pub trait Flex {
    fn measure(&self, line_width: f32) -> FlexMeasure;
    
    fn flex(&self, factor: f32) -> FlexMeasure {
        let m = self.measure(0.);
        FlexMeasure {
            width: m.width,
            shrink: m.shrink / factor,
            stretch: m.stretch * factor,
            height: m.height
        }
    }
}



#[derive(Debug)]
pub enum Entry<O: Output> {
    /// A single word (sequence of glyphs)
    Word(O::Word),
    
    /// Punctuation ('"', ',', '.', '-', …)
    /// is positioned in the margin if at the beginning or end of the line
    Punctuation(O::Word),
    
    
    Object(Box<Object>),
    
    /// Continue on the next line (fill)
    Linebreak(bool),
    
    /// (breaking, measure)
    Space(bool, FlexMeasure),
    
    /// Somtimes there are different possiblites of representing something.
    /// A Branch solves this by splitting the stream in two parts.
    /// The default path is taken by skipping the specified amount of entries.
    /// The other one by following the next items.
    ///
    /// normal items
    /// BranchEntry(3)
    ///   branched item 1
    ///   branched item 2
    /// BranchExit(1)
    ///   normal item 1
    /// both sides joined here
    BranchEntry(usize),
    
    /// Each BranchEntry is followed by BranchExit. It specifies the number of
    /// items to skip.
    BranchExit(usize),
    
    Style(Rc<Style<O>>),
    
    /// a reference to something.
    /// location can be queried, once the main layout is complete
    Anchor(Counter)
}

#[derive(Debug)]
pub enum Counter {
    None,       // not counted, fails if it can't be positioned exactly
    Page,       // numbers are unique on each page; but different pages share the same numbers
    Chapter,    // unique to each chapter
    Document    // unique to the whole document
}

pub type StreamVec<O> = Vec<Entry<O>>;

#[derive(Copy, Clone)]
pub struct Atom<'a> {
    pub left:   Glue,
    pub right:  Glue,
    pub text:   &'a str
}
impl<'a> Atom<'a> {
    pub fn normal(t: &'a str) -> Atom<'a> {
        Atom {
            left:   Glue::space(),
            right:  Glue::space(),
            text:   t
        }
    }
}
impl<'a> fmt::Display for Atom<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.left, self.text, self.right)
    }
}


pub trait BranchGenerator<'a> {
    fn add(&mut self, &mut FnMut(&mut Writer));
}

pub trait Object: Debug {
    fn measure(&self, primary: Length) -> FlexMeasure;
    fn show(&self, out: &mut Surface);
    fn glue(&self) -> (Glue, Glue);
}

#[derive(PartialEq, Eq, Hash)]
pub enum NodeType {
    Section(String), // a block with the given name
    Header(String),  // the header for the block of the given name
    Body(String),    // the body for "
    Default
}

pub trait Writer {
    // a single word, ignoring glue
    fn word(&mut self, word: Atom);
    
    // 
    fn punctuation(&mut self, p: Atom);
    
    fn branch(&mut self, &mut FnMut(&mut BranchGenerator));
    
    fn promote(&mut self, glue: Glue);
    
    fn object(&mut self, _item: Box<Object>) {}
    
    fn with(&mut self, name: &str,
        head: &mut FnMut(&mut Writer),
        body: &mut FnMut(&mut Writer)
    );
}

pub trait Surface {
    fn primary(&self) -> Length;
    fn secondary(&self) -> Option<Length>;
}

