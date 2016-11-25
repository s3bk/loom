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
        use itertools::Itertools;
        write!(
            self.file,
            "layout([\n{}\n]);\n",
            stream.iter().map(|item| {
                match item {
                    &StreamItem::Word(ref w) => format!("  [0, {:?}]", w.s),
                    &StreamItem::Linebreak(f) => format!("  [1, {:?}]", f),
                    &StreamItem::Space(b, m) => format!("  [2, {:?}, {}]", b, m),
                    &StreamItem::BranchEntry(s) => format!("  [3, {}]", s),
                    &StreamItem::BranchExit(s) => format!("  [4, {:?}]", s),
                }
            }).join(",\n")
        );
    }
}
