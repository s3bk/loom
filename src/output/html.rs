use layout::*;
use std::error::Error;
use std::fmt::{Debug, self};
use output::Output;
use std::path::Path;
use std::io::Write;
use std::fs::File;
use sxd_document::{Package, dom};

pub struct HtmlOutput {
    package: Package
}

struct HtmlFakeBranchGen<'a, 'b> where 'b: 'a {
    w:      &'a mut HtmlWriter<'b>,
    first:  bool
}
impl<'a, 'b> BranchGenerator<'a> for HtmlFakeBranchGen<'a, 'b> where 'b: 'a {
    fn add(&mut self, f: &mut FnMut(&mut Writer)) {
        if self.first {
            self.first = false;
            f(self.w);
        }
    }
}
pub struct HtmlWriter<'a> {
    element:    dom::Element<'a>,
    state:      Glue
}
impl<'a> HtmlWriter<'a> {
    fn add<F, C>(&self, f: F) where
    F: FnOnce(dom::Document<'a>) -> C,
    C: Into<dom::ChildOfElement<'a>>
    {
        self.element.append_child(f(self.element.document()))
    }
    pub fn new(out: &mut HtmlOutput) -> HtmlWriter {
        let doc = out.doc();
        let html = doc.create_element("html");
        let body = doc.create_element("body");
        let article = doc.create_element("article");
        
        body.append_child(article);
        html.append_child(body);
        doc.root().append_child(html);
        
        HtmlWriter {
            element:    article,
            state:      Glue::None
        }
    }
    pub fn finish<W: Write>(self, out: &mut W) {
        use sxd_document::writer::format_document;
        format_document(&self.element.document(), out);
    }
    
    fn add_glue(&mut self, glue: Glue) {
        match self.state | glue {
            Glue::None => {},
            Glue::Space { breaking: true, .. } => {
                self.add(|d| d.create_text(" "));
            },
            Glue::Space { breaking: false, .. } => {
                self.add(|d| d.create_text(" "));
            },
            Glue::Newline { .. } => {
                self.add(|d| d.create_element("br"));
            }
        }
    }
}

impl<'a> Writer for HtmlWriter<'a> {
    fn word(&mut self, word: Atom) {
        self.add_glue(word.left);
        self.add(|d| d.create_text(word.text));
        self.state = word.right;
    }
    
    fn punctuation(&mut self, p: Atom) {
        self.add_glue(p.left);
        self.add(|d| d.create_text(p.text));
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
    
    fn object(&mut self, item: Box<Object>) {}
    
    fn section(&mut self, f: &mut FnMut(&mut Writer), name: &str) {
        self.add(|d| {
            let s = d.create_element("section");
            s.set_attribute_value("name", name);
            
            f(&mut HtmlWriter {
                element:    s,
                state:      Glue::None
            });
            s
        })
    }
}

impl HtmlOutput {
    pub fn new() -> HtmlOutput {
        let package = Package::new();
        
        HtmlOutput {
            package:    package
        }
    }
    fn doc(&self) -> dom::Document {
        self.package.as_document()
    }
}
