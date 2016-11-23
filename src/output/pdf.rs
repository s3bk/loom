use output::{Output, VectorOutput};
use layout::{Flex, FlexMeasure, Word};
use std::error::Error;

use pdf;

struct PdfOutput<'a> {
    canvas: pdf::Canvas<'a>
}

#[derive(Clone)]
struct PdfFont {
    font:   pdf::FontRef,
    size:   f32
}
struct UnscaledPdfFont {
    font:   pdf::FontRef
}

#[derive(Clone)]
struct PdfMeasuredWord {
    text:   String,
    width:  f32,
    height: f32,
}
impl Flex for PdfMeasuredWord {
    fn width(&self, _: f32) -> f32 {
        self.width
    }
    fn shrink(&self, _: f32) -> f32 {
        self.width
    }
    fn stretch(&self, _: f32) -> f32 {
        self.width
    }
    fn height(&self, _: f32) -> f32 {
        self.height
    }
}
impl Word for PdfMeasuredWord {}

impl<'a> Output for PdfOutput<'a> {
    type Font = PdfFont;
    type Word = PdfMeasuredWord;
    
    fn measure(f: &PdfFont, word: &str) -> PdfMeasuredWord {
        use pdf::FontSource;
        
        PdfMeasuredWord {
            text:   word.to_owned(),
            width:  f.font.get_width(f.size, word),
            height: f.size
        }
    }
    fn default_font(&mut self) -> PdfFont {
        PdfFont {
            font:   self.canvas.get_font(pdf::BuiltinFont::Times_Roman),
            size:   12.
        }
    }
}

impl<'a> VectorOutput for PdfOutput<'a> {
    type UnscaledFont = UnscaledPdfFont;
    
    fn scale(&self, f: &UnscaledPdfFont, size: f32) -> PdfFont {
        PdfFont {
            font:   f.font.clone(),
            size:   size
        }
    }

    fn use_font(&mut self, file: &str) -> Result<UnscaledPdfFont, Box<Error>> {
        Ok(UnscaledPdfFont {
            font:   self.canvas.get_font(pdf::BuiltinFont::Times_Roman)
        })
    }
}
