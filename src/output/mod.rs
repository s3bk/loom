use std::error::Error;
use std::sync::Arc;
use std::ops::{BitOr};
use layout::{FlexMeasure, Surface};
use std::fmt::Debug;
use num::Zero;
use units::*;
use io::{Io, AioError, File};
use futures::Future;

pub trait Output {
    // 
    //type Measure: Clone + Debug + Sized + Zero;
    type Word: Clone + Debug + Sized;
    type Font: Clone + Sized;
    type UnscaledFont;
    type Surface: Surface;
    
    fn measure(&Self::Font, &str) -> Self::Word;
    
    /// It is highly recommended to implement caching.
    /// Vector based and above
    fn use_font(&self, io: &Io, file: &File)
     -> Box<Future<Item=Self::UnscaledFont, Error=AioError>>;
    
    fn scale(&self, &Self::UnscaledFont, size: Length) -> Self::Font;
    fn measure_word(&Self::Word, line_width: Length) -> FlexMeasure;
    fn measure_space(&Self::Font, scale: Scale) -> FlexMeasure;
    
    fn draw_word(surface: &mut Self::Surface, pos: Point, word: &Self::Word);
}


#[cfg(feature = "output_png")]
pub mod png;

#[cfg(feature = "output_pdf")]
pub mod pdf;

#[cfg(feature = "output_html")]
pub mod html;

