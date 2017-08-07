use layout::{FlexMeasure, Surface, Style};
use std::fmt::Debug;
use units::*;
use io;

pub trait Output: Sized {
    // 
    //type Measure: Clone + Debug + Sized + Zero;
    type Word: Clone + Debug + Sized;
    type Font: Clone + Sized + Debug;
    type UnscaledFont;
    type Surface: Surface;
    
    fn measure(&Self::Font, &str) -> Self::Word;
    
    fn use_font_data(&self, data: io::Data) -> Self::UnscaledFont;
    
    fn scale(&self, &Self::UnscaledFont, size: Length) -> Self::Font;
    fn measure_word(&Self::Word, line_width: Length) -> FlexMeasure;
    fn measure_space(&Self::Font, scale: Scale) -> FlexMeasure;
    
    fn draw_word(surface: &mut Self::Surface, pos: Point, word: &Self::Word);

    fn style(&self, name: &str) -> Option<&Style<Self>>;
    fn style_or_default(&self, name: &str) -> &Style<Self> {
        match self.style(name) {
            Some(s) => s,
            None => self.style("default").expect("failed to get default style")
        }
    }
}


#[cfg(feature = "output_png")]
pub mod png;

#[cfg(feature = "output_pdf")]
pub mod pdf;

#[cfg(feature = "output_html")]
pub mod html;

