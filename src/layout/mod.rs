use std::fmt::{self, Debug};
use units::*;

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
pub enum Entry<W> {
    /// A single word (sequence of glyphs)
    Word(W),
    
    /// Punctuation ('"', ',', '.', '-', …)
    /// is positioned in the margin if at the beginning or end of the line
    Punctuation(W),
    
    
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
}

pub type StreamVec<Word> = Vec<Entry<Word>>;

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

pub trait Writer {
    // a single word, ignoring glue
    fn word(&mut self, word: Atom);
    
    // 
    fn punctuation(&mut self, p: Atom);
    
    fn branch(&mut self, &mut FnMut(&mut BranchGenerator));
    
    fn promote(&mut self, glue: Glue);
    
    fn object(&mut self, item: Box<Object>);
    
    fn section(&mut self, f: &mut FnMut(&mut Writer), name: &str);
}

pub trait Surface {
    fn primary(&self) -> Length;
    fn secondary(&self) -> Option<Length>;
}


// private mods
mod glue;
mod paragraph;
mod generic_writer;
mod flex;
mod style;

pub use self::glue::Glue;
pub use self::paragraph::ParagraphLayout;
pub use self::generic_writer::{GenericWriter};
pub use self::flex::FlexMeasure;
pub use self::style::Style;
