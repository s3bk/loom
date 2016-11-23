use layout::{Flex, FlexMeasure, ParagraphLayout, StreamVec, Word};
use image::{GrayImage, Luma};
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use rusttype;
use std::fmt::{Debug, self};
use output::{Output, VectorOutput};


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
    fn space(&self) -> FlexMeasure {
        self.measure(" ").flex(0.)
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
        self.inner.width
    }
    fn shrink(&self, line_width: f32) -> f32 {
        self.inner.width
    }
    fn stretch(&self, line_width: f32) -> f32 {
        self.inner.width
    }
    fn height(&self, line_width: f32) -> f32 {
        let m = self.inner.font.v_metrics(self.inner.scale);
        m.line_gap + m.ascent - m.descent
    }
}
impl Debug for RustTypeWord {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "RustTypeWord \"{}\"", self.inner.text)
    }
}
impl Word for RustTypeWord {}
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
    
    pub fn render(&mut self, stream: &StreamVec<RustTypeWord>, width: f32) -> GrayImage {
        use std::time::SystemTime;
        
        fn m(label: &str, t0: SystemTime, t1: SystemTime) {
            let d = t1.duration_since(t0).unwrap();
            println!("{} {:01}.{:09}s", label, d.as_secs(), d.subsec_nanos());
        }
        
        let t2 = SystemTime::now();
        let margin_v = 10.0;
        let margin_h = 10.0;
        
        let lines = ParagraphLayout::<PngOutput>::new(stream, width).run();
        let height: f32 = lines.iter().map(|l| l.height).sum();
        let mut image = GrayImage::from_pixel(
            (width + 2. * margin_h) as u32,
            (height + 2. * margin_v) as u32,
            Luma { data: [255u8] }
        );
        
        let t3 = SystemTime::now();
        m("layout:     ", t2, t3);
        
        let mut y = margin_v;
        for line in lines.iter() {
            y += line.height;
            for &(ref word, x) in line.words.iter() {
                word.draw_at(&mut image, (x+margin_h, y));
            }
        }
        let t4 = SystemTime::now();
        m("drawing:    ", t3, t4);
        
        image
    }
}

impl Output for PngOutput {
    type Word = RustTypeWord;
    type Font = RustTypeFont;
    
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
}
impl VectorOutput for PngOutput {
    type UnscaledFont = UnscaledRustTypeFont;
    
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
}
