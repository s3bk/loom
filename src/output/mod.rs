use std::error::Error;
use std::sync::Arc;
use std::ops::{BitOr};
use layout::{Word, FlexMeasure};

enum FormatRequest {
    Color,
    Size,
    Font,
}

pub trait Output {
    type Word: Word;
    type Font: Clone;
    
    fn measure(&Self::Font, &str) -> Self::Word;
    fn default_font(&mut self) -> Self::Font;
}


pub trait VectorOutput: Output {
    type UnscaledFont;
    
    /// It is highly recommended to implement caching.
    /// Vector based and above
    fn use_font(&mut self, file: &str) -> Result<Self::UnscaledFont, Box<Error>>;
    
    fn scale(&self, &Self::UnscaledFont, size: f32) -> Self::Font;
}

pub mod png;
pub mod pdf;
