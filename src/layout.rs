use std::sync::Arc;
use std::iter::FromIterator;
use std::ops::{AddAssign};
use std::fmt::Debug;
use typeset::{MeasuredWord};

// to flex or not to flex?
#[allow(unused_variables)]
pub trait Flex : Debug + Sync + Send {
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
#[allow(unused_variables)]
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
    /// The default path is taken by skipping the specified amount of entries.
    /// The other one by following the next items.
    ///
    /// normal items
    /// BranchEntry(3)
    ///   branched item 1
    ///   branched item 2
    /// BranchExit(1)
    ///   normal item 1
    /// both sides joined here
    BranchEntry(usize),
    
    /// Each BranchEntry is followed by BranchExit. It specifies the number of
    /// items to skip.
    BranchExit(usize)
}


#[derive(Clone, Debug)]
pub struct TokenStream {
    buf: Vec<StreamItem>
}
impl TokenStream {

    pub fn new() -> TokenStream {
        TokenStream { buf: vec![] }
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    
    fn add(&mut self, i: StreamItem) {
        self.buf.push(i);
    }
    
    pub fn extend(&mut self, t: &TokenStream) -> &mut TokenStream {
        self.buf.extend_from_slice(&t.buf);
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
        match self.buf.last() {
            Some(&StreamItem::Linebreak) => {},
            _ => self.add(StreamItem::Linebreak)
        }
        self
    }
    
    pub fn branch(&mut self, a: &TokenStream, b: &TokenStream) -> &mut TokenStream {
        self.add(StreamItem::BranchEntry(b.buf.len() + 1));
        self.extend(b);
        self.add(StreamItem::BranchExit(a.buf.len()));
        self.extend(a);
        self
    }
    
    pub fn branch_many<I>(&mut self, default: TokenStream, others: I)
    where I: Iterator<Item=TokenStream> {
        let mut others: Vec<TokenStream> = others.collect();
        
        if others.len() == 0 {
            self.extend(&default);
            return;
        }
        
        while others.len() > 1 {
            for n in 0 .. others.len() / 2 {
                let mut merged = TokenStream::new();
                {
                    let b = others.pop().unwrap();
                    let ref a = others[n];
                    merged.branch(a, &b);
                }
                others[n] = merged;
            }
        }
        self.branch(&default, &others[0]);
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct LineBreak {
    prev:   usize,
    path:   u64, // one bit for each branch taken (1) or not (0)
    factor: f32,
    score:  f32
}
type Entry = Option<LineBreak>;

pub struct ParagraphLayout<'a> {
    items:      &'a Vec<StreamItem>,
    width:      f32,
}
pub struct Line {
    pub words:  Vec<(Arc<MeasuredWord>, f32)>,
    pub height: f32
}

struct LineContext {
    measure:    FlexMeasure,
    path:       u64, // one bit for each branch on this line
    begin:      usize, // begin of line or branch
    pos:        usize, // calculation starts here
    score:      f32, // score at pos
    branches:   u8 // number of branches so far (<= 64)
}

impl<'a> ParagraphLayout<'a> {
    pub fn new(s: &'a TokenStream, width: f32) -> ParagraphLayout<'a> {
        ParagraphLayout {
            items: &s.buf,
            width: width
        }
    }
    
    pub fn run(&mut self) -> Vec<Line> {
        use std::iter::repeat;
        
        let limit = self.items.len();
        let mut nodes: Vec<Entry> = repeat(None).take(limit+1).collect();
        nodes[0] = Some(LineBreak::default());
        let mut last = 0;
        
        for start in 0 .. limit {  
            let node = nodes[start];
            match node {
                Some(b) => {
                    last = self.complete_line(
                        &mut nodes,
                        LineContext {
                            measure:    FlexMeasure::zero(),
                            path:       0,
                            score:      b.score,
                            begin:      start,
                            pos:        start,
                            branches:   0
                        }
                    );
                },
                None => {}
            }
        }
        /*
        for (n, node) in nodes.iter().take(limit).enumerate() {
            println!("{:4}  {:?}", n, node);
            println!("      {:?}", self.items[n]);
        }
        println!("{:4}  {:?}", limit, nodes[limit]);
        println!("last: {}", last);
        */
        if last == 0 {
            return vec![];
        }
        
        let mut steps = vec![];
        
        while last > 0 {
            //println!("node {:3} {:?}", end, self.nodes[end]);
            match nodes[last] {
                Some(b) => {
                    //println!("Line from {} to {}", start, end-1);
                    steps.push((b, last-1));
                    last = b.prev;
                },
                _ => unreachable!()
            }
        }
        
        let mut lines = Vec::with_capacity(steps.len());
        for &(b, end) in steps.iter().rev() {
            let mut measure = FlexMeasure::zero();
            let mut words = vec![];
            let mut pos = b.prev;
            let mut branches = 0;
            while pos < end {
                let node = self.items[pos].clone();
                match node {
                    StreamItem::Word(w) => {
                        let x = measure.at(b.factor);
                        measure += w.measure(self.width);
                        words.push((w, x));
                    },
                    StreamItem::Space(s) |
                    StreamItem::BreakingSpace(s) => {
                        measure += s.measure(self.width)
                    },
                    StreamItem::BranchEntry(len) => {
                        if b.path & (1<<branches) == 0 {
                            // not taken
                            pos += len;
                        }
                        branches += 1;
                    },
                    StreamItem::BranchExit(skip) => pos += skip,
                    _ => {}
                }
                pos += 1;
            }
            
            lines.push(Line {
                height: measure.height,
                words:  words
            });
        }
        
        lines
    }
    
    
    fn complete_line(&self, nodes: &mut Vec<Entry>, c: LineContext) -> usize {
        let mut last = c.begin;
        let mut c = c;
        
        while c.pos < self.items.len() {
            let n = c.pos;
            let ref item = self.items[n];
            match item {
            //match self.items[n] {
                &StreamItem::Word(ref w) => {
                    c.measure += w.measure(self.width);
                },
                &StreamItem::Space(ref s) => {
                    c.measure += s.measure(self.width);
                },
                &StreamItem::BreakingSpace(ref s) => {
                    // breaking case:
                    // width is not added (yet)!
                    if self.maybe_update(&c, &mut nodes[n+1]) {
                        last = n+1;
                    }
                    
                    // non-breaking case:
                    // add width now.
                    c.measure += s.measure(self.width);
                }
                &StreamItem::Linebreak => {
                    if self.maybe_update(&c, &mut nodes[n+1]) {
                        last = n+1;
                    }
                    break;
                },
                &StreamItem::BranchEntry(len) => {
                    use std::cmp;
                    // b 
                    let b_last = self.complete_line(
                        nodes,
                        LineContext {
                            pos:        n + 1,
                            path:       c.path | (1 << c.branches),
                            branches:   c.branches + 1,
                            ..          c
                        }
                    );
                    last = cmp::max(last, b_last);
                    
                    // a follows here
                    c.pos += len;
                    c.branches += 1;
                },
                &StreamItem::BranchExit(skip) => {
                    c.pos += skip;
                }
            }
            
            if c.measure.shrink > self.width {
                break; // too full
            }
            
            c.pos += 1;
        }
        
        last
    }
    
    
    fn maybe_update(&self, c: &LineContext, node: &mut Entry) -> bool {
        let width = self.width;
        let ref m = c.measure;
        
        if width < m.shrink || m.stretch <= m.width {
            return false;
        }
    
        let delta = width - m.width; // d > 0 => stretch, d < 0 => shrink
        let factor = delta / (if delta >= 0. { m.stretch - m.width } else { m.width - m.shrink });
        
        let break_score = c.score - factor * factor;
        match *node {
            None => {
                *node = Some(LineBreak {
                    prev:   c.begin,
                    path:   c.path,
                    factor: factor,
                    score:  break_score
                } );
            },
            Some(other)
            if break_score > other.score => {
                *node = Some(LineBreak {
                    prev:   c.begin,
                    path:   c.path,
                    factor: factor,
                    score:  break_score
                } );
            },
            _ => {}
        }
        true
    }
}

