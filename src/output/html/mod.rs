use layout::*;
use std::io::Write;
use serde_json;
use std::collections::HashMap;
use marksman_escape::Escape;
use wheel::Directory;
use futures::Future;
use io::{self};
use super::super::LoomError;

const HEAD: &'static str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE xhtml PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html>
    <head>
        <link rel="stylesheet" href="style.css" />
    </head>
    <body>
"#;

const TAIL: &'static str = "\
    </body>
</html>
";

fn tag(s: &str) -> &str {
    s
}
fn attr_val(s: &str) -> &str {
    s
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct HtmlStyle {
    tag:        String,
    header:     Option<String>
}

pub struct HtmlOutput {
    styles:     HashMap<String, HtmlStyle>
}
impl HtmlOutput {
    pub fn load(root: &Directory) -> Box<Future<Item=HtmlOutput, Error=LoomError>> {
        use std::str;
        
        box io::open_read(&root, "html.style")
        .map(|data| HtmlOutput::new(str::from_utf8(&data).expect("invalid utf8")))
    }
    pub fn new(styles: &str) -> HtmlOutput {
        HtmlOutput {
            styles: serde_json::from_str(styles).expect("failed to parse styles")
        }
    }
}


struct HtmlFakeBranchGen<'a, 'b, W> where 'b: 'a, W: Write + 'b {
    w:      &'a mut HtmlWriter<'b, W>,
    first:  bool
}
impl<'a, 'b, W> BranchGenerator<'a> for HtmlFakeBranchGen<'a, 'b, W>
where 'b: 'a, W: Write + 'a
{
    fn add(&mut self, f: &mut FnMut(&mut Writer)) {
        if self.first {
            self.first = false;
            f(self.w);
        }
    }
}
pub struct HtmlWriter<'a, W: Write + 'a> {
    state:      Glue,
    output:     &'a HtmlOutput,
    writer:     &'a mut W,
}
fn write_glue<W: Write>(w: &mut W, glue: Glue) {
    match glue {
        Glue::None => Ok(()),
        Glue::Space { breaking: true, scale }  =>
            if scale == 1.0 {
                write!(w, "<sp></sp>")
            } else {
                write!(w, r#"<sp style="flex-stretch: {};"></sp>"#, scale)
            },
            Glue::Space { breaking: false, scale } => if scale == 1.0 {
                write!(w, "<nbs></nbs>")
            } else {
                write!(w, r#"<nbs style="flex-stretch: {};"></nbs>"#, scale)
            },
        Glue::Newline { .. } => write!(w, "<nl></nl>")
    }.unwrap()
}

impl<'a, W: Write + 'a> HtmlWriter<'a, W> {
    pub fn new(out: &'a mut HtmlOutput, writer: &'a mut W) -> HtmlWriter<'a, W> {
        writer.write(HEAD.as_bytes()).unwrap();

        HtmlWriter {
            state:      Glue::None,
            output:     out,
            writer:     writer
        }
    }
    pub fn finish(self) {
        self.writer.write(TAIL.as_bytes()).unwrap();
    }
    
    fn add_glue(&mut self, glue: Glue) {
        write_glue(&mut self.writer, self.state | glue);
    }
    
    fn add_text(&mut self, text: &str) {
        self.writer.write(b"<w>").unwrap();

        for escaped in Escape::new(text.bytes()) {
            self.writer.write(&[escaped]).unwrap();
        }
        self.writer.write(b"</w>").unwrap();
    }
}

impl<'a, W: Write + 'a> Writer for HtmlWriter<'a, W> {
    fn word(&mut self, word: Atom) {
        self.add_glue(word.left);
        self.add_text(word.text);
        self.state = word.right;
    }
    
    fn punctuation(&mut self, p: Atom) {
        self.add_glue(p.left);
        self.add_text(p.text);
        self.state = p.right;
    }
    
    fn branch(&mut self, f: &mut FnMut(&mut BranchGenerator)) {
        f(&mut HtmlFakeBranchGen {
            w:      self,
            first:  true
        });
    }
    
    fn promote(&mut self, glue: Glue) {
        self.state |= glue;
    }
    
    fn with(&mut self, name: &str,
        head: &mut FnMut(&mut Writer),
        body: &mut FnMut(&mut Writer)
    ) {
        let style = self.output.styles.get(name)
        .unwrap_or_else(|| self.output.styles.get("*").expect("* style missing"));
        
        // finish glue
        write_glue(&mut self.writer, self.state);
        self.state = Glue::None;
        
        write!(self.writer, r#"<{} name="{}">"#, tag(&style.tag), attr_val(name)).unwrap();
        
        if let Some(ref header) = style.header {
            write!(self.writer, "<{}>", tag(header)).unwrap();
            head(self);
            write!(self.writer, "</{}>", tag(header)).unwrap();
            
        } else {
            head(self);
        }
        body(self);
        
        write!(self.writer, "</{}>", tag(&style.tag)).unwrap();
    }
}
