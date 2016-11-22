use std::error::Error;
use std::sync::Arc;
use std::ops::{BitOr};
use layout::{Flex, TokenStream};

pub enum Glue {
    None,
    Space {
        breaking:   bool,
        scale:      f32
    },
    Newline {
        fill:       bool
    }
}
impl BitOr for Glue {
    fn bitor(self, rhs: Glue) -> Glue {
        use self::Glue::*;
        
        match (self, rhs) {
            // Glue::None wins over anything else
            (None, _) | (_, None) => None,
            
            (Space { breaking: false, .. }, Newline { .. }) |
            (Newline { .. }, Space { breaking: false, .. }) => {
                panic!("Newline and NonBreaking requested");
            },
            
            // NonBreaking wins over Breaking
            (Space { breaking: false, scale: a }, Space { breaking: true,  scale: b }) |
            (Space { breaking: true,  scale: a }, Space { breaking: false, scale: b })
             => Space { breaking: false, scale: a | b },
            
            // Newline wins over Breaking
            (Newline { fill: a }, Space { breaking: true, .. }) |
            (Space { breaking: true, .. }, Newline { fill: a })
             => Newline { fill: a },
            
            (Space { breaking: true, scale: a }, Space { breaking: true,  scale: b })
             => Space { breaking: true, scale: a | b }
        }
    }
}
impl Glue {
    fn space() -> Glue {
        Glue::Space { breaking: true, scale: 1.0 }
    }
    fn nbspace() -> Glue {
        Glue::Space { breaking: false, scale: 1.0 }
    }
}
enum FormatRequest {
    Color,
    Size,
    Font,
}

pub struct Word<'a> {
    text:   &'a str,
    left:   Glue,
    right:  Glue
}

pub trait Output {
    type Word;
    type Font;
    
    fn measure(&self, &Self::Font, word: &str) -> Self::Word;
}

pub struct Writer<O: Output> {
    pub output: O,
    state:      Glue,
    layout:     TokenStream<O>,
}
impl<O: Output> Writer<O> {
    #[inline(always)]
    pub fn push<F>(&mut self, left: Glue, right: Glue, f: F) where
    F: FnOnce(&mut TokenStream<O>, &O)
    {
        match self.state | left {
            Glue::Newline { fill: f }
             => self.layout.newline(f),
            Glue::Space { breaking: b, scale: s }
             => self.layout.space(b, f),
            Glue::None => ()
        }
        f(&mut self.layout, &self.output);
        
        self.state = right;
    }
    
    #[inline(always)]
    pub fn push_word(&mut self, left: Glue, right: Glue, w: O::Word) {
        self.push(left, right, move |l, o| l.word(w));
    }
    
    #[inline(always)]
    pub fn promote(&mut self, glue: Glue) {
        self.break_state = self.break_state | glue;
    }
}

pub trait VectorOutput: Output {
    type UnscaledFont;
    
    /// It is highly recommended to implement caching.
    /// Vector based and above
    fn use_font(&mut self, file: &str) -> Result<Self::UnscaledFont, Box<Error>>;
    fn default_font(&mut self) -> Self::UnscaledFont;
    
    fn space(&self, &Self::Font) -> Arc<Flex>;
    fn scale(&self, &Self::UnscaledFont, size: f32) -> Arc<Self::Font>;
}

pub mod png;
pub mod pdf;
