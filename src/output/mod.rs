use std::error::Error;
use std::sync::Arc;
use std::ops::{BitOr};
use layout::{Flex};
use std::fmt::Debug;

enum FormatRequest {
    Color,
    Size,
    Font,
}

pub trait Output {
    type Measure: Clone + Debug + Sized;
    type Word: Clone + Debug + Sized;
    type Font: Clone + Sized;
    
    fn measure(&Self::Font, &str) -> Self::Word;
    fn measure_space(&Self::Font, scale: f32) -> Self::Measure;
    fn default_font(&mut self) -> Self::Font;
}

pub trait VectorOutput: Output {
    type UnscaledFont;
    type WordV: Flex;
    type MeasureV: Clone + Flex;
    
    /// It is highly recommended to implement caching.
    /// Vector based and above
    fn use_font(&mut self, file: &str) -> Result<Self::UnscaledFont, Box<Error>>;
    
    fn scale(&self, &Self::UnscaledFont, size: f32) -> Self::Font;
}

#[cfg(feature = "output_png")]
mod png;

#[cfg(feature = "output_pdf")]
mod pdf;

#[cfg(feature = "output_html")]
mod html;

#[cfg(feature = "output_png")]
pub use self::png::PngOutput;

#[cfg(feature = "output_pdf")]
pub use self::pdf::PdfOutput;

#[cfg(feature = "output_html")]
pub use self::html::HtmlOutput;
