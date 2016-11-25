use layout::*;
use output::Output;
use std::iter::Extend;

struct GenericBranchGen<'a, O: Output + 'a> {
    parent: &'a mut GenericWriter<O>,
    branches: Vec<StreamVec<O::Word, O::Measure>>
}
impl<'a, O: Output> BranchGenerator<'a> for GenericBranchGen<'a, O> {
    fn add(&mut self, f: &mut FnMut(&mut Writer)) {
        let mut w = self.parent.dup();
        f(&mut w);
        self.branches.push(w.stream);
        self.parent.state |= w.state;
    }
}
impl<'a, O: Output> Drop for GenericBranchGen<'a, O> {
    fn drop(&mut self) {
        self.parent.push_branch(self.branches.drain(..));
    }
}

pub struct GenericWriter<O: Output> {
    state:      Glue,
    stream:     StreamVec<O::Word, O::Measure>,
    font:       O::Font
}

// careful with the arguments.. they all have the same type!
fn merge<W, M>(out: &mut StreamVec<W, M>, a: StreamVec<W, M>, b: StreamVec<W, M>) {
    out.push(StreamItem::BranchEntry(b.len() + 1));
    out.extend(b);
    out.push(StreamItem::BranchExit(a.len()));
    out.extend(a);
}
impl<O: Output> GenericWriter<O> {
    pub fn new(font: O::Font) -> GenericWriter<O> {
        GenericWriter {
            state:  Glue::None,
            stream: Vec::new(),
            font:   font
        }
    }
    fn dup(&mut self) -> GenericWriter<O> {
        GenericWriter {
            stream: Vec::new(),
            state:  Glue::None,
            font:   self.font.clone(),
        }
    }
    
    pub fn finish(&mut self) -> &StreamVec<O::Word, O::Measure> {
        self.write_glue(Glue::Newline { fill: false });
        &self.stream
    }
    
    fn push_branch<I>(&mut self, mut ways: I) where I: Iterator<Item=StreamVec<O::Word, O::Measure>> {
        if let Some(default) = ways.next() {
            let mut others: Vec<StreamVec<O::Word, O::Measure>> = ways.collect();
            
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
             => self.stream.push(StreamItem::Linebreak(f)),
            Glue::Space { breaking: b, scale: s }
             => self.stream.push(StreamItem::Space(b, O::measure_space(&self.font, s))),
            Glue::None => ()
        }
    }
    
    #[inline(always)]
    fn push<F>(&mut self, left: Glue, right: Glue, f: F) where
    F: FnOnce(&mut StreamVec<O::Word, O::Measure>, &O::Font)
    {
        self.write_glue(left);
        f(&mut self.stream, &self.font);
        
        self.state = right;
    }
}   
impl<O: Output> Writer for GenericWriter<O> {
    fn branch(&mut self, left: Glue, right: Glue, ways: usize,
    f: &mut FnMut(&mut BranchGenerator))
    {
        self.write_glue(left);
        f(&mut GenericBranchGen {
            parent:     self,
            branches:   Vec::with_capacity(ways)
        });
        self.state = right;
    }

    #[inline(always)]
    fn word(&mut self, word: Atom) {
        self.push(word.left, word.right, move |s, f|
            s.push(StreamItem::Word(O::measure(f, word.text)))
        );
    }
    
    #[inline(always)]
    fn promote(&mut self, glue: Glue) {
        self.state |= glue;
    }
}
 
