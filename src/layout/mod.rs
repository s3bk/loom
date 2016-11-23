use std::sync::Arc;
use std::iter::FromIterator;
use std::ops::{AddAssign, Mul};
use std::fmt::Debug;
use output::{Output};

// to flex or not to flex?
#[allow(unused_variables)]
pub trait Flex {
    fn stretch(&self, line_width: f32) -> f32 { 0.0 }
    fn shrink(&self, line_width: f32) -> f32 { 0.0 }
    fn width(&self, line_width: f32) -> f32;
    fn height(&self, line_width: f32) -> f32 { 0.0 }
    
    fn measure(&self, line_width: f32) -> FlexMeasure {
        FlexMeasure {
            shrink:     self.shrink(line_width),
            stretch:    self.stretch(line_width),
            width:      self.width(line_width),
            height:     self.height(line_width)
        }
    }
    
    fn flex(&self, factor: f32) -> FlexMeasure {
        let w = self.width(0.);
        FlexMeasure {
            width: w,
            shrink: w / factor,
            stretch: w * factor,
            height: self.height(0.)
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub struct FlexMeasure {
    shrink:     f32,
    stretch:    f32,
    width:      f32,
    height:     f32
}

impl FlexMeasure {
    pub fn zero() -> FlexMeasure {
        FlexMeasure {
            width: 0.,
            stretch: 0.,
            shrink: 0.,
            height: 0.
        }        
    }
    /// factor = -1 => self.shrink,
    /// factor =  0 => self.width,
    /// factor = +1 => self.stretch
    pub fn at(&self, factor: f32) -> f32 {
        (if factor < 0. {
            (self.width - self.shrink)
        } else {
            (self.stretch - self.width)
        } * factor + self.width)
    }
}
impl AddAssign for FlexMeasure {
    fn add_assign(&mut self, rhs: FlexMeasure) {
        self.width += rhs.width;
        self.stretch += rhs.stretch;
        self.shrink += rhs.shrink;
        self.height = self.height.max(rhs.height);
    }
}
impl Mul<f32> for FlexMeasure {
    type Output = FlexMeasure;
    
    fn mul(self, f: f32) -> FlexMeasure {
        FlexMeasure {
            width:      self.width * f,
            stretch:    self.stretch * f,
            shrink:     self.shrink * f,
            height:     self.height
        }        
    }
}

#[derive(Clone, Debug)]
pub enum StreamItem<W: Clone + Flex> {
    /// A single word (sequence of glyphs)
    Word(W),
    
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
    BranchExit(usize)
}

pub type StreamVec<W: Word> = Vec<StreamItem<W>>;
pub trait Word: Flex + Clone {}

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
pub trait BranchGenerator<'a> {
    fn add(&mut self, &mut FnMut(&mut Writer));
}

pub trait Writer {
    // a single word, ignoring glue
    fn word(&mut self, word: Atom);
    
    fn branch(&mut self, left: Glue, right: Glue, ways: usize, &mut FnMut(&mut BranchGenerator));
    
    fn promote(&mut self, glue: Glue);
}

// private mods
mod glue;
mod paragraph;
mod generic_writer;

pub use self::glue::Glue;
pub use self::paragraph::ParagraphLayout;
pub use self::generic_writer::{GenericWriter};
