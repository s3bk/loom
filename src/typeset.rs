use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::cell::RefCell;
use rusttype;
use image::{GrayImage, Luma};
use layout::{Flex};
use std::fmt::{Debug, self};

pub trait MeasuredWord : Flex {
    fn draw_at(&self, s: &mut GrayImage, p: (f32, f32));
}

pub trait Font {
    /// It is highly recommended to implement caching.
    fn measure(&self, word: &str) -> Arc<MeasuredWord>;
    fn space(&self) -> Arc<Flex>;
}

pub trait UnscaledFont {
    fn scale(&self, size: f32) -> Arc<Font>;
}

/// Abstract Type Engine that is used to produce visible text
pub trait TypeEngine {
    // for now ... needs more functionality
    fn load_font(&mut self, file: &str) -> Result<Box<UnscaledFont>, Box<Error>>;
}




// concrete type //
struct RustTypeFont {
    font:   rusttype::Font<'static>,
    scale:  rusttype::Scale,
    cache:  RefCell<HashMap<String, Arc<RustTypeWord>>>,
}
impl RustTypeFont {
    fn _measure(&self, word: &str) -> Arc<RustTypeWord> {
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
        
        let w = Arc::new(RustTypeWord {
            font:       self.font.clone(),
            scale:      self.scale,
            glyphs:     glyph_list,
            width:      width,
            text:       word.to_owned()
        });
        self.cache.borrow_mut().insert(word.to_owned(), w.clone());
        w
    }
}
impl Font for RustTypeFont {
    fn measure(&self, word: &str) -> Arc<MeasuredWord> {
        self._measure(word) as Arc<MeasuredWord>
    }
    
    fn space(&self) -> Arc<Flex> {
        self._measure(" ") as Arc<Flex>
    }
}
struct RustTypeWord {
    font:       rusttype::Font<'static>,
    scale:      rusttype::Scale,
    glyphs:     Vec<(rusttype::GlyphId, f32)>,
    width:      f32,
    text:       String
}

fn saturate(pixel: &mut Luma<u8>, increment: u8) {
    let ref mut v = pixel.data[0];
    *v = v.saturating_sub(increment);
}

impl MeasuredWord for RustTypeWord {
    fn draw_at(&self, image: &mut GrayImage, pos: (f32, f32)) {
        use rusttype::point;
        
        let it = self.glyphs.iter()
        .map(|&(id, dx)|(self.font.glyph(id).unwrap().scaled(self.scale).positioned(point(pos.0+dx, pos.1))));
        for g in it {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, v| {
                    saturate(
                        image.get_pixel_mut(
                            (x as i32 + bb.min.x) as u32,
                            (y as i32 + bb.min.y) as u32
                        ),
                        ((v * 255.) as u8)
                    );
                });
            }
        }
    }
}
#[allow(unused_variables)]
impl Flex for RustTypeWord {
    fn width(&self, line_width: f32) -> f32 {
        self.width
    }
    fn shrink(&self, line_width: f32) -> f32 {
        self.width
    }
    fn stretch(&self, line_width: f32) -> f32 {
        self.width
    }
    fn height(&self, line_width: f32) -> f32 {
        let m = self.font.v_metrics(self.scale);
        m.line_gap + m.ascent - m.descent
    }
}
impl Debug for RustTypeWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "RustTypedWord \"{}\"", self.text)
    }
}

struct UnscaledRustTypeFont {
    font:   rusttype::Font<'static>
}
impl UnscaledFont for UnscaledRustTypeFont {
    fn scale(&self, size: f32) -> Arc<Font> {
        Arc::new(RustTypeFont {
            font:   self.font.clone(),
            scale:  rusttype::Scale::uniform(size),
            cache:  RefCell::new(HashMap::new())
        }) as Arc<Font>
    }
}

pub struct RustTypeEngine {}

impl TypeEngine for RustTypeEngine {
    fn load_font(&mut self, path: &str) -> Result<Box<UnscaledFont>, Box<Error>> {
        use std::fs::File;
        use std::io::Read;
        
        let mut f = try!(File::open(path));
        let mut data = Vec::<u8>::new();
        try!(f.read_to_end(&mut data));
    
        Ok(Box::new(UnscaledRustTypeFont {
            font: rusttype::FontCollection::from_bytes(data).font_at(0).unwrap()
        }) as Box<UnscaledFont>)
    }
}
impl RustTypeEngine {
    pub fn new() -> Box<TypeEngine> {
        Box::new(RustTypeEngine {}) as Box<TypeEngine>
    }
    pub fn default() -> Box<UnscaledFont> {
        Box::new(UnscaledRustTypeFont {
            font: rusttype::FontCollection::from_bytes(
                include_bytes!("../data/fonts/LiberationSerif-Regular.ttf") as &'static [u8]
            ).font_at(0).unwrap()
        }) as Box<UnscaledFont>
    }
}
