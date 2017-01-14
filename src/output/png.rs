use layout::{Flex, FlexMeasure, ParagraphLayout, StreamVec, Surface};
use image::{GrayImage, Luma};
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::Path;
use std::io;
use rusttype;
use std::fmt::{Debug, self};
use output::{Output};
use units::*;

#[derive(Clone)]
pub struct RustTypeFont {
    font:   rusttype::Font<'static>,
    scale:  rusttype::Scale,
    cache:  RefCell<HashMap<String, RustTypeWord>>,
}
impl RustTypeFont {
    fn measure(&self, word: &str) -> RustTypeWord {
        if let Some(w) = self.cache.borrow().get(word) {
            return w.clone();
        }
        assert!(word.len() > 0);
        
        let glyphs: Vec<(rusttype::GlyphId, rusttype::HMetrics)> = self.font.glyphs_for(word.chars())
            .map(|g| (g.id(), g.scaled(self.scale).h_metrics()))
            .collect();
        
        assert!(glyphs.len() > 0);
        
        let mut glyph_list = Vec::with_capacity(glyphs.len());
        glyph_list.push((glyphs[0].0, 0.));
        
        let mut prev_id = glyphs[0].0;
        let mut width = glyphs[0].1.advance_width;
        for &(id, h_metrics) in glyphs[1 ..].iter() {
            width += self.font.pair_kerning(self.scale, prev_id, id);
            glyph_list.push((id, width));
            width += h_metrics.advance_width;
            prev_id = id;
        }
        
        let w = RustTypeWord {
            inner:  Rc::new(RustTypeWordInner {
                font:       self.font.clone(),
                scale:      self.scale,
                glyphs:     glyph_list,
                width:      width,
                text:       word.to_owned()
            })
        };
        self.cache.borrow_mut().insert(word.to_owned(), w.clone());
        w
    }
}

struct RustTypeWordInner {
    font:       rusttype::Font<'static>,
    scale:      rusttype::Scale,
    glyphs:     Vec<(rusttype::GlyphId, f32)>,
    width:      f32,
    text:       String
}
#[derive(Clone)]
pub struct RustTypeWord {
    inner:  Rc<RustTypeWordInner>
}
impl Debug for RustTypeWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.text)
    }
}
fn saturate(pixel: &mut Luma<u8>, increment: u8) {
    let ref mut v = pixel.data[0];
    *v = v.saturating_sub(increment);
}

impl RustTypeWordInner {
    fn draw_at(&self, image: &mut GrayImage, pos: (f32, f32)) {
        use rusttype::point;
        
        let it = self.glyphs.iter()
        .map(|&(id, dx)|(self.font.glyph(id).unwrap().scaled(self.scale).positioned(point(pos.0+dx, pos.1))));
        for g in it {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    let px = x as i32 + bb.min.x;
                    let py = y as i32 + bb.min.y;
                    if px >= 0 && px < image.width() as i32 &&
                       py >= 0 && py < image.height() as i32 {
                        saturate(
                            image.get_pixel_mut(
                                px as u32,
                                py as u32
                            ),
                            ((v * 255.) as u8)
                        );
                    }
                });
            }
        }
    }
}
#[allow(unused_variables)]
impl Flex for RustTypeWord {
    fn measure(&self, line_width: f32) -> FlexMeasure {
        let m = self.inner.font.v_metrics(self.inner.scale);
        
        FlexMeasure {
            shrink:     self.inner.width,
            stretch:    self.inner.width,
            width:      self.inner.width,
            height:     m.line_gap + m.ascent - m.descent
        }
    }
}
impl RustTypeWord {
    fn draw_at(&self, image: &mut GrayImage, pos: (f32, f32)) {
        self.inner.draw_at(image, pos);
    }
}

pub struct UnscaledRustTypeFont {
    font:   rusttype::Font<'static>
}

pub struct PngOutput {}
impl PngOutput {
    pub fn new() -> PngOutput {
        PngOutput {}
    }
    
    pub fn surface(&self, size: Size) -> PngSurface {
        PngSurface {
            image: GrayImage::from_pixel(
                size.0 as u32,
                size.1 as u32,
                Luma { data: [255u8] }
            )
        }
    }
}

impl Output for PngOutput {
    type Word = RustTypeWord;
    type Font = RustTypeFont;
    type UnscaledFont = UnscaledRustTypeFont;
    type Surface = PngSurface;
    
    fn measure(font: &RustTypeFont, word: &str) -> RustTypeWord {
        font.measure(word)
    }
    
    
    fn default_font(&mut self) -> RustTypeFont {
        self.scale(&UnscaledRustTypeFont {
            font: rusttype::FontCollection::from_bytes(
                include_bytes!(
                    "../../data/fonts/LiberationSerif-Regular.ttf"
                ) as &'static [u8]
            ).font_at(0).unwrap()
        }, 18.)
    }
    
    
    fn measure_space(font: &RustTypeFont, scale: f32) -> FlexMeasure {
        font.measure(" ").flex(2.0) * scale
    }
    fn measure_word(w: &RustTypeWord, line_width: f32) -> FlexMeasure {
        w.measure(line_width)
    }
    fn scale(&self, font: &UnscaledRustTypeFont, size: f32) -> RustTypeFont {
        RustTypeFont {
            font:   font.font.clone(),
            scale:  rusttype::Scale::uniform(size),
            cache:  RefCell::new(HashMap::new())
        }
    }

    fn use_font(&mut self, file: &str) -> Result<UnscaledRustTypeFont, Box<Error>> {
        use std::fs::File;
        use std::io::Read;
        
        let mut f = try!(File::open(file));
        let mut data = Vec::<u8>::new();
        try!(f.read_to_end(&mut data));
    
        Ok(UnscaledRustTypeFont {
            font: rusttype::FontCollection::from_bytes(data).font_at(0).unwrap()
        })
    }
    
    fn draw_word(surface: &mut PngSurface, pos: Point, word: &RustTypeWord) {
        word.draw_at(&mut surface.image, pos);
    }
}

pub struct PngSurface {
    image: GrayImage
}
impl PngSurface {
    pub fn save(&self, p: &Path) -> io::Result<()> {
        self.image.save(p)
    }
}
impl Surface for PngSurface {
    fn primary(&self) -> Length {
        self.image.width() as Length
    }
    fn secondary(&self) -> Option<Length> {
        Some(self.image.height() as Length)
    }
    
    //fn region(&self, rect: Rect) -> Surface<'a>;
}
