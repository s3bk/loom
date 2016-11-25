use layout::{Flex, FlexMeasure, StreamVec, StreamItem};
use std::error::Error;
use std::fmt::{Debug, self};
use output::Output;
use std::path::Path;
use std::io::Write;
use std::fs::File;

#[derive(Debug, Clone)]
pub struct HtmlWord {
    s:  String
}

#[derive(Clone)]
pub struct HtmlFont {

}

pub struct HtmlOutput {
    file:   File,
}

impl Output for HtmlOutput {
    type Word = HtmlWord;
    type Font = HtmlFont;
    type Measure = f32;
    
    fn measure(_: &HtmlFont, s: &str) -> HtmlWord {
        HtmlWord { s: s.to_owned() }
    }
    fn measure_space(_: &HtmlFont, scale: f32) -> f32 {
        scale
    }
    fn default_font(&mut self) -> HtmlFont {
        HtmlFont {}
    }
}

impl HtmlOutput {
    pub fn new(path: &Path) -> HtmlOutput {
        HtmlOutput {
            file: File::create(path).expect("could not create file")
        }
    }

    pub fn render(&mut self, stream: &StreamVec<HtmlWord, f32>) {
        write!(self.file, "\
<html>
    <body>
        <section id=\"target\">
        <section id=\"source\">");
        for item in stream.iter() {
            match item {
                &StreamItem::Word(ref w) => {
                    write!(self.file, "<word>{}</word>", w.s);
                }
                _ => {}
            }
        }
        write!(self.file, "</section>
    </body>
</html>
"       );
    }
}
