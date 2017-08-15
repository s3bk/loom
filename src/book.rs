use units::*;
use io::Yarn;
use output::Output;
use layout::*;
use tuple::T2;
use std::fmt::Debug;

pub struct PageMetrics {
    pub width:          Length,  // overall width of the final page
    pub height:         Length, // overall height of the final page
    pub margin_inner:   Length, // space between the binding and the text
    pub margin_outer:   Length, // space between the text and the edge of the page
    pub margin_top:     Length,
    pub margin_bottom:  Length
}
impl PageMetrics {
    pub fn text_width(&self) -> Length {
        self.width - self.margin_inner - self.margin_outer
    }
    pub fn text_height(&self) -> Length {
        self.height - self.margin_top - self.margin_bottom
    }
    pub fn page_size(&self) -> T2<Length, Length> {
        T2(self.width, self.height)
    }
    pub fn margin_left(&self, nr: usize) -> Length {
        if nr % 2 == 0 {
            self.margin_outer
        } else {
            self.margin_inner
        }
    }
}

pub struct Book {
    content: Yarn,
//    margin: Margin,
    page:   PageMetrics
}

pub struct Pages<O: Output> {
    pages: Vec<O::Surface>
}

impl Book {
    fn render<O>(&self, output: &O) -> Pages<O> where O: Output + Debug {
        let mut w = GenericWriter::new(output);
        self.content.layout(&mut w);
        
        let layout = ColumnLayout::new(w.finish(), self.page.text_width(), self.page.text_height());
        
        let mut page_nr = 1;
        let mut surfaces = Vec::new();
        
        for column in layout.columns() {
            let offset = T2(self.page.margin_left(page_nr), self.page.margin_top);
            let mut marginnotes = Vec::new();
        
            let mut surface = output.surface(self.page.page_size());
            for (dy, line) in column {
                for (dx, e) in line {
                    match e {
                        Item::Word(word) => O::draw_word(&mut surface, T2(dx, dy) + offset, word),
                        Item::Anchor(data) => marginnotes.push((dy, data))
                    }
                }
            }
            
            // margin
            for (y, data) in marginnotes {
                let layout = ParagraphLayout::new(data, self.page.margin_outer);
                for (dy, line) in layout.lines() {
                    let offset = T2(self.page.width - self.page.margin_outer, self.page.margin_top + y);
                    for (dx, e) in line {
                        match e {
                            Item::Word(word) => O::draw_word(&mut surface, T2(dx, dy) + offset, word),
                            _ => {}
                        }
                    }
                }
            }

            surfaces.push(surface);
//            ::std::fs::File::create(&format!("{}_{:03}.png", name.to_str().unwrap(), i)).unwrap()
//            .write_all(&surface.encode()).unwrap();
        }

        Pages {
            pages: surfaces,
        }
    }
}
