use output::{Output, VectorOutput};
use layout::FlexMeasure;
use std::error::Error;

use pdf;

struct PdfOutput {}

struct PdfFont {
    font:   pdf::FontRef,
    size:   f32
}
struct UnscaledPdfFont {
    font:   pdf::FontRef
}
struct PdfMeasuredWord {
    text:   String,
    width:  f32
}

impl Output for PdfOutput {
    type Font = PdfFont;
    type Word = PdfMeasuredWord;
    
    fn measure(&self, f: &PdfFont, word: &str) -> PdfMeasuredWord {
        PdfMeasuredWord {
            text:   word.to_owned(),
            width:  f.get_width(f.size, word)
        }
    }
}

impl VectorOutput for PdfOutput {
    type UnscaledFont = UnscaledPdfFont;
    
    fn scale(&self, f: &UnscaledPdfFont, size: f32) -> PdfFont {
        PdfFont {
            font:   f.font,
            size:   size
        }
    }

    fn use_font(&mut self, file: &str) -> Result<UnscaledPdfFont, Box<Error>> {
        Ok(UnscaledPdfFont {
            font:   pdf::BuiltinFont::Times_Roman
        })
    }
    fn default_font(&mut self) -> UnscaledPdfFont {
        Ok(UnscaledPdfFont {
            font:   pdf::BuiltinFont::Times_Roman
        })
    }
}
