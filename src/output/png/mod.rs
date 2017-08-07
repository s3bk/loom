use layout::{Flex, FlexMeasure, Surface, Style};
use image::{GrayImage, Luma, Pixel};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use rusttype;
use std::fmt::{Debug, self};
use output::Output;
use config::Config;
use futures::future::{self, Future};
use units::*;
use io::{self, open_read};
use super::super::LoomError;
use serde_json;
use istring::IString;

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
impl Debug for RustTypeFont {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RustTypeFont")
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
        .map(|&(id, dx)| (
            self.font.glyph(id).unwrap()
            .scaled(self.scale)
            .positioned(point(pos.0+dx, pos.1))
        ));
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
impl UnscaledRustTypeFont {
    fn load(data: io::Data) -> UnscaledRustTypeFont {
        UnscaledRustTypeFont {
            font: rusttype::FontCollection::from_bytes(data.to_vec()).font_at(0).expect("failed to read font")
        }
    }
}
#[derive(Debug)]
pub struct PngOutput {
    styles: HashMap<IString, Style<PngOutput>>
}
impl PngOutput {
    pub fn load(config: Config) -> Box<Future<Item=PngOutput, Error=LoomError>>
    {
        box open_read(&config.style_dir, "png.style")
        .and_then(move |data| {
            let mut font_cache: HashMap<String, _> = HashMap::new();
            
            #[derive(Deserialize, Clone)]
            struct RawStyle {
                font_size:  Option<f32>,
                leading:    Option<f32>,
                par_indent: Option<f32>,
                font_name:  Option<String>
            }
            
            let raw_map: HashMap<String, RawStyle> = serde_json::from_slice(&data).unwrap();
            for value in raw_map.values() {
                if let Some(ref name) = value.font_name {
                    font_cache.entry(name.clone()).or_insert_with(|| {
                        open_read(&config.font_dir, name)
                    });
                }
            }
        
            future::join_all(
                font_cache.into_iter()
                .map(|(name, future)| future.map(move |r| (name, r)))
            )
            .map(move |items| (raw_map, items))
        })
        .map(move |(raw_map, items)| {
            // translate files into fonts
            let fonts: HashMap<String, Rc<UnscaledRustTypeFont>> = items.into_iter()
            .map(|(font_name, data)|
                (font_name, Rc::new(UnscaledRustTypeFont::load(data)))
            ).collect();
            
            let mut output = PngOutput {
                styles: HashMap::with_capacity(raw_map.len())
            };
            let default_raw = raw_map.get("default").cloned().expect("no default style");
            let default_font = &fonts[default_raw.font_name.as_ref().expect("no default font_name")];
            let default_size = default_raw.font_size.expect("no default font_size");
            
            let default = Style::<PngOutput> {
                font_size:  default_size,
                leading:    default_raw.leading.unwrap_or(default_size * 1.5),
                font:       output.scale(default_font, default_size),
                par_indent: default_raw.par_indent.unwrap_or(0.)
            };
            
            for (name, raw) in raw_map.into_iter() {
                let size = raw.font_size.unwrap_or(default.font_size);
                let font = raw.font_name.map(|name| &fonts[&name]).unwrap_or(default_font);
                let style = Style {
                    font_size:  size,
                    leading:    raw.leading.unwrap_or(default.leading),
                    font:       output.scale(font, size),
                    par_indent: raw.par_indent.unwrap_or(default.par_indent)
                };
                output.styles.insert(name.into(), style);
            }

            output
        })
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

    fn use_font_data(&self, data: io::Data) -> UnscaledRustTypeFont {
        UnscaledRustTypeFont::load(data)
    }
    
    fn draw_word(surface: &mut PngSurface, pos: Point, word: &RustTypeWord) {
        word.draw_at(&mut surface.image, pos);
    }

    fn style(&self, name: &str) -> Option<&Style<PngOutput>> {
        self.styles.get(name)
    }
}

pub struct PngSurface {
    image: GrayImage
}
impl PngSurface {
    pub fn encode(&self) -> Vec<u8> {
        use image::png::PNGEncoder;
        let mut data = Vec::new();
        let ref i = self.image;
        
        PNGEncoder::new(&mut data)
        .encode(i, i.width(), i.height(), Luma::<u8>::color_type())
        .expect("failed to encode PNG");
        
        data
    }
    pub fn image(&self) -> &GrayImage {
        &self.image
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
