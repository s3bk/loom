use std::sync::Arc;
use std::iter;
use std::ops::{AddAssign};
use std::fmt::Debug;
use typeset::{MeasuredWord};

// to flex or not to flex?
pub trait Flex : Debug {
    fn stretch(&self, line_width: f32) -> f32 { 0.0 }
    fn shrink(&self, line_width: f32) -> f32 { 0.0 }
    fn width(&self, line_width: f32) -> f32;
    fn height(&self, line_width: f32) -> f32 { 0.0 }
    
    fn measure(&self, line_width: f32) -> FlexMeasure {
        FlexMeasure {
            shrink:     self.shrink(line_width),
            stretch:    self.stretch(line_width),
            width:      self.width(line_width),
            height:     self.height(line_width)
        }
    }
    
    fn flex(&self, factor: f32) -> Arc<Flex> {
        let w = self.width(0.);
        Arc::new(FlexMeasure {
            width: w,
            shrink: w / factor,
            stretch: w * factor,
            height: self.height(0.)
        }) as Arc<Flex>
    }

}

#[derive(Copy, Clone, Debug)]
pub struct FlexMeasure {
    shrink:     f32,
    stretch:    f32,
    width:      f32,
    height:     f32
}
impl Flex for FlexMeasure {
    fn stretch(&self, line_width: f32) -> f32 { self.stretch }
    fn shrink(&self, line_width: f32) -> f32 { self.shrink }
    fn width(&self, line_width: f32) -> f32 { self.width }
    fn height(&self, line_width: f32) -> f32 { self.height }
}

impl FlexMeasure {
    pub fn zero() -> FlexMeasure {
        FlexMeasure {
            width: 0.,
            stretch: 0.,
            shrink: 0.,
            height: 0.
        }        
    }
    /// factor = -1 => self.shrink,
    /// factor =  0 => self.width,
    /// factor = +1 => self.stretch
    pub fn at(&self, factor: f32) -> f32 {
        (if factor < 0. {
            (self.width - self.shrink)
        } else {
            (self.stretch - self.width)
        } * factor + self.width)
    }
}
impl AddAssign for FlexMeasure {
    fn add_assign(&mut self, rhs: FlexMeasure) {
        self.width += rhs.width;
        self.stretch += rhs.stretch;
        self.shrink += rhs.shrink;
        self.height = self.height.max(rhs.height);
    }
}

/// Send is reqired
/// All indexing has to be relative
/// (to enable inserting, replacing and deleting parts of the steam.)
///
/// As simple "space" (_) is the most common item in the stream,
/// is should be represented by only one item.
#[derive(Clone, Debug)]
enum StreamItem {
    /// A single word (sequence of glyphs)
    Word(Arc<MeasuredWord>),
    
    /// Continue on the next line
    Linebreak,
    
    /// non-breaking space
    Space(Arc<Flex>),
    
    /// breakable space
    BreakingSpace(Arc<Flex>),
    
    /// Somtimes there are different possiblites of representing something.
    /// A Branch solves this by splitting the stream in two parts.
    /// The lengths of both paths are the arguments to Branch.
    /// The third argument is the score for the second branch.
    Branch(u32, u32, f32)
}


pub struct TokenStream {
    s: Vec<StreamItem>,
}
impl TokenStream {
    pub fn new() -> TokenStream {
        TokenStream { s: vec![] }
    }
    
    fn add(&mut self, i: StreamItem) {
        self.s.push(i)
    }
    
    pub fn extend(&mut self, t: &TokenStream) -> &mut TokenStream {
        self.s.extend_from_slice(&t.s);
        self
    }
    
    pub fn word(&mut self, w: Arc<MeasuredWord>) -> &mut TokenStream {
        self.add(StreamItem::Word(w));
        self
    }
    
    pub fn nbspace(&mut self, f: Arc<Flex>) -> &mut TokenStream {
        self.add(StreamItem::Space(f));
        self
    }
    
    pub fn space(&mut self, f: Arc<Flex>) -> &mut TokenStream {
        self.add(StreamItem::BreakingSpace(f));
        self
    }
    
    pub fn newline(&mut self) -> &mut TokenStream {
        self.add(StreamItem::Linebreak);
        self
    }
    
    pub fn branch(&mut self, a: TokenStream, b: TokenStream, penalty: f32) -> &mut TokenStream {
        self.add(StreamItem::Branch(a.s.len() as u32, b.s.len() as u32, penalty));
        self.s.extend_from_slice(&a.s);
        self.s.extend_from_slice(&b.s);
        self
    }
}

#[derive(Copy, Clone, Debug)]
enum Entry {
    Empty,
    LineBreak(usize, f32, f32),
    Branch(usize, bool)
}

fn maybe_update(node: &mut Entry, start: usize, start_score: f32, text_width: f32, m: FlexMeasure) -> bool {
    if text_width < m.shrink || m.stretch <= m.width {
        return false;
    }
    
    let delta = text_width - m.width; // d > 0 => stretch, d < 0 => shrink
    let factor = delta / (if delta >= 0. { m.stretch - m.width } else { m.width - m.shrink });
    
    let break_score = start_score - factor * factor;
    match *node {
        Entry::Empty => {
            *node = Entry::LineBreak(start, factor, break_score);
        },
        Entry::LineBreak(_, _, other_score)
        if break_score > other_score => {
            *node = Entry::LineBreak(start, factor, break_score);
        },
        _ => {}
    }
    true
}

pub struct ParagraphLayout {
    items:      Vec<StreamItem>,
    pos:        usize,
    width:      f32,
    nodes:      Vec<Entry>,
}
impl ParagraphLayout {
    pub fn new(s: TokenStream, width: f32) -> ParagraphLayout {
        let N = s.s.len();
        ParagraphLayout {
            items: s.s,
            pos: 0,
            width: width,
            nodes: iter::repeat(Entry::Empty).take(N+1).collect()
        }
    }
}
pub struct Line {
    pub words:  Vec<(Arc<MeasuredWord>, f32)>,
    pub height: f32
}
impl Iterator for ParagraphLayout {
    type Item = Vec<Line>;
    
    fn next(&mut self) -> Option<Vec<Line>> {
        let N = self.items.len();
        self.nodes[self.pos] = Entry::LineBreak(0, 0.0, 0.0);
        
    // get top node
        let mut next = self.pos;
  'a:   for start in self.pos .. N {
            let node = self.nodes[start];
            match node {
                Entry::Empty => {},
                Entry::LineBreak(prev, _, start_score) => {
                    let mut measure = FlexMeasure::zero();
                    
                    for n in start .. N {
                        let ref item = self.items[n];
                        match item {
                        //match self.items[n] {
                            &StreamItem::Word(ref w) => {
                                measure += w.measure(self.width);
                            },
                            &StreamItem::Space(ref s) => {
                                measure += s.measure(self.width);
                            },
                            &StreamItem::BreakingSpace(ref s) => {
                                // breaking case:
                                // width is not added (yet)!
                                maybe_update(&mut self.nodes[n+1], start, start_score, self.width, measure);
                                
                                // non-breaking case:
                                // add width now.
                                measure += s.measure(self.width);
                            }
                            &StreamItem::Linebreak => {
                                if maybe_update(&mut self.nodes[n+1], start, start_score, self.width, measure) {
                                    next = n + 1;
                                }
                                continue 'a;
                            },
                            _ => {}
                        }
                        
                        if measure.shrink > self.width {
                            break; // too full
                        }
                    }
                },
                Entry::Branch(prev, taken) => {}
            }
        }
        
        if next == self.pos {
            return None;
        }
        
        for (n, node) in self.nodes.iter().take(N).enumerate() {
            println!("{:4}  {:?}", n, node);
            println!("      {:?}", self.items[n]);
        }
        println!("{:4}  {:?}", N, self.nodes[N]);
        
        let mut end = next;
        let mut steps = vec![];
        
        while end > self.pos {
            println!("node {:3} {:?}", end, self.nodes[end]);
            match self.nodes[end] {
                Entry::LineBreak(start, factor , _) => {
                    println!("Line from {} to {}", start, end-1);
                    steps.push((start, end-1, factor));
                    end = start;
                },
                _ => {
                    unreachable!();
                }
            }
        }
        
        let mut lines = Vec::with_capacity(steps.len());
        for &(start, end, factor) in steps.iter().rev() {
            let mut measure = FlexMeasure::zero();
            let mut words = vec![];
            for node in self.items[start .. end].iter() {
                match node {
                    &StreamItem::Word(ref w) => {
                        words.push((w.clone(), measure.at(factor)));
                        measure += w.measure(self.width)
                    },
                    &StreamItem::Space(ref s) |
                    &StreamItem::BreakingSpace(ref s) => {
                        measure += s.measure(self.width)
                    },
                    _ => {}
                }
            }
            
            lines.push(Line {
                height: measure.height,
                words:  words
            });
            self.pos = end;
        }
        
        Some(lines)
    }
}

