use layout::*;
use std::io::Write;
use serde_json;
use std::collections::HashMap;
use marksman_escape::Escape;
use jenga::place_iter;

const HEAD: &'static str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
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
fn glue_str(glue: Glue) -> &'static str {
    match glue {
        Glue::None => "",
        Glue::Space { breaking: true, .. } => " ",
        Glue::Space { breaking: false, .. } => " ",
        Glue::Newline { .. } => "<br />"
    }
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
        self.writer.write(glue_str(self.state | glue).as_bytes()).unwrap();
    }
    
    fn add_text(&mut self, text: &str) {
        use std::str;
        place_iter(
            Escape::new(text.bytes()), |escaped| {
                println!("{} -> {:?}", text, str::from_utf8(escaped).unwrap());
                self.writer.write(escaped).unwrap();
            }
        ).unwrap();
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
        self.writer.write(glue_str(self.state).as_bytes()).unwrap();
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
