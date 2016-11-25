use output::{Output, VectorOutput};
use layout::{Flex, FlexMeasure, Word, StreamVec, ParagraphLayout};
use std::error::Error;
use std::fmt::{self, Debug};
use std::path::Path;

use pdf;

pub struct PdfOutput {
    pdf: pdf::Pdf
}
impl PdfOutput {
    pub fn new(path: &Path) -> PdfOutput {
        PdfOutput {
            pdf: pdf::Pdf::create(path.to_str().unwrap()).unwrap()
        }
    }
    pub fn render(&mut self, stream: &StreamVec<PdfMeasuredWord>, width: f32) {
        let margin_v = 10.0;
        let margin_h = 10.0;
        
        let lines = ParagraphLayout::<PdfOutput>::new(stream, width).run();
        let height: f32 = lines.iter().map(|l| l.height).sum();
        
        self.pdf.render_page(width + 2. * margin_h, height + 2. * margin_v, |canvas| {
            let mut y = margin_v;
            for line in lines.iter() {
                y += line.height;
                for &(ref word, x) in line.words.iter() {
                    canvas.text(|text| {
                        text.set_font(&word.font, word.size);
                        text.pos(x+margin_h, y);
                        text.show(&word.text);
                        Ok(())
                    });
                }
            }
            Ok(())
        });
    }
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
    size:   f32,
    font:   pdf::FontRef,
}
impl Debug for PdfMeasuredWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
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
        self.size
    }
}
impl Word for PdfMeasuredWord {}

impl Output for PdfOutput {
    type Font = PdfFont;
    type Word = PdfMeasuredWord;
    
    fn measure(f: &PdfFont, word: &str) -> PdfMeasuredWord {
        use pdf::FontSource;
        
        PdfMeasuredWord {
            text:   word.to_owned(),
            width:  f.font.get_width(f.size, word),
            size: f.size,
            font:   f.font.clone()
        }
    }
    fn default_font(&mut self) -> PdfFont {
        PdfFont {
            font:   self.canvas.get_font(pdf::BuiltinFont::Times_Roman),
            size:   12.
        }
    }
}

impl VectorOutput for PdfOutput {
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
