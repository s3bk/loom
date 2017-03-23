use layout::*;
use output::Output;
use std::iter::Extend;
use layout::style::{Stylist};

struct GenericBranchGen<'a, O: Output + 'a> {
    parent: &'a GenericWriter<'a, O>,
    branches: Vec<(StreamVec<O>, Glue)>
}
impl<'a, O: Output + 'static> BranchGenerator<'a> for GenericBranchGen<'a, O> {
    fn add(&mut self, f: &mut FnMut(&mut Writer)) {
        let mut w = self.parent.dup();
        f(&mut w);
        self.branches.push((w.stream, w.state));
    }
}
pub struct GenericWriter<'a, O: Output + 'a> {
    state:      Glue,
    stream:     StreamVec<O>,
    styler:     &'a Stylist<O>,
    style:      &'a Style<O>
}

// careful with the arguments.. they all have the same type!
fn merge<O: Output>(out: &mut StreamVec<O>, mut a: StreamVec<O>, mut b: StreamVec<O>) {
    if a.len() == 0 {
        out.extend(b);
    } else if b.len() == 0 {
        out.extend(a);
    } else {
        let equal_end = match (a.last().unwrap(), b.last().unwrap()) {
            (&Entry::Space(a_break, a_measure), &Entry::Space(b_break, b_measure)) =>
                (a_break == b_break) && (a_measure == b_measure),
            _ => false
        };
        
        let end_sym = if equal_end {
            a.pop();
            b.pop()
        } else {
            None
        };

        out.push(Entry::BranchEntry(b.len() + 1));
        out.extend(b);
        out.push(Entry::BranchExit(a.len()));
        out.extend(a);
        
        if let Some(end) = end_sym {
            out.push(end);
        }
    }
}
impl<'a, O: Output + 'static> GenericWriter<'a, O> {
    pub fn new(styler: &'a Stylist<O>) -> GenericWriter<'a, O> {
        GenericWriter {
            state:  Glue::None,
            stream: Vec::new(),
            style:  styler.default(),
            styler: styler
        }
    }
    fn dup(&self) -> GenericWriter<O> {
        GenericWriter {
            stream: Vec::new(),
            ..      *self
        }
    }
    
    pub fn finish(&mut self) -> &StreamVec<O> {
        self.write_glue(Glue::Newline { fill: false });
        &self.stream
    }
    
    fn push_branch<I>(&mut self, mut ways: I) where I: Iterator<Item=StreamVec<O>> {
        if let Some(default) = ways.next() {
            let mut others: Vec<StreamVec<O>> = ways.collect();
            
            if others.len() == 0 {
                self.stream.extend(default);
                return;
            }
            
            while others.len() > 1 {
                for n in 0 .. others.len() / 2 {
                    use std::mem;
                    // TODO use with_capacity
                    let mut merged = StreamVec::new();
                    let mut tmp = Vec::new();
                    
                    mem::swap(&mut tmp, others.get_mut(n).unwrap());
                    merge(&mut merged, tmp, others.pop().unwrap());
                    others[n] = merged;
                }
            }
            merge(&mut self.stream, default, others.pop().unwrap());
        }
    }
    
    #[inline(always)]
    fn write_glue(&mut self, left: Glue) {
        match self.state | left {
            Glue::Newline { fill: f }
             => self.stream.push(Entry::Linebreak(f)),
            Glue::Space { breaking: b, scale: s }
             => self.stream.push(Entry::Space(b, O::measure_space(self.style.font(), s))),
            Glue::None => ()
        }
    }
    
    #[inline(always)]
    fn push<F>(&mut self, left: Glue, right: Glue, f: F) where
    F: FnOnce(&mut StreamVec<O>, &O::Font)
    {
        self.write_glue(left);
        f(&mut self.stream, self.style.font());
        
        self.state = right;
    }
}   
impl<'a, O: Output + 'static> Writer for GenericWriter<'a, O> {
    fn branch(&mut self, f: &mut FnMut(&mut BranchGenerator))
    {
        let mut branches = {
            let mut gen = GenericBranchGen {
                parent:     self,
                branches:   Vec::new()
            };
            f(&mut gen);
        
            gen.branches
        };
        let mut glue = Glue::any();
        self.push_branch(branches.drain(..).map(|(v, s)| {
            glue |= s;
            v
        }));
        self.state = glue;
        // FIXME
        //self.state = right;
    }

    #[inline(always)]
    fn word(&mut self, word: Atom) {
        self.push(word.left, word.right, move |s, f|
            s.push(Entry::Word(O::measure(f, word.text)))
        );
    }
        
    fn punctuation(&mut self, p: Atom) {
        self.push(p.left, p.right, move |s, f|
            s.push(Entry::Punctuation(O::measure(f, p.text)))
        );
    }
    
    fn object(&mut self, _item: Box<Object>) {
    
    }
    
    #[inline(always)]
    fn promote(&mut self, glue: Glue) {
        self.state |= glue;
    }
    
    fn with(&mut self, name: &NodeType, f: &mut FnMut(&mut Writer)) {
        let old_style = self.style;
        self.style = self.styler.get(name);
        f(self);
        self.style = old_style;
    }
}
 
