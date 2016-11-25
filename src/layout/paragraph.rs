use layout::{StreamItem, StreamVec, FlexMeasure, Flex};
use output::{Output, VectorOutput};
use std::clone::Clone;
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, Default)]
struct LineBreak {
    prev:   usize,
    path:   u64, // one bit for each branch taken (1) or not (0)
    factor: f32,
    score:  f32
}
type Entry = Option<LineBreak>;

pub struct ParagraphLayout<'a, W: 'a, M: 'a> {
    items:      &'a StreamVec<W, M>,
    width:      f32,
}
pub struct Line<Word> {
    pub words:  Vec<(Word, f32)>,
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

impl<'a, W: Flex + Debug + Clone, M: Flex + Debug + Clone> ParagraphLayout<'a, W, M>  {
    pub fn new(s: &'a StreamVec<W, M>, width: f32)
     -> ParagraphLayout<'a, W, M>
    {
        ParagraphLayout {
            items: s,
            width: width
        }
    }
    
    pub fn run(&mut self) -> Vec<Line<W>> {
        use std::iter::repeat;
        
        let limit = self.items.len();
        let mut nodes: Vec<Entry> = repeat(None).take(limit+1).collect();
        nodes[0] = Some(LineBreak::default());
        let mut last = 0;
        
        for start in 0 .. limit {
            match nodes[start] {
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
        
        for (n, node) in nodes.iter().take(limit).enumerate() {
            println!("{:4}  {:?}", n, node);
            println!("      {:?}", self.items[n]);
        }
        println!("{:4}  {:?}", limit, nodes[limit]);
        println!("last: {}", last);
        
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
                    StreamItem::Space(_, s) => {
                        measure += s.measure(self.width);
                    },
                    StreamItem::BranchEntry(len) => {
                        if b.path & (1<<branches) == 0 {
                            // not taken
                            pos += len;
                        }
                        branches += 1;
                    },
                    StreamItem::BranchExit(skip) => pos += skip,
                    StreamItem::Linebreak(_) => unreachable!()
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
        use layout::Flex;
        let mut last = c.begin;
        let mut c = c;
        
        while c.pos < self.items.len() {
            let n = c.pos;
            let ref item = self.items[n];
            match item {
                &StreamItem::Word(ref w) => {
                    c.measure += w.measure(self.width);
                },
                &StreamItem::Space(breaking, ref s) => {
                    if breaking {
                        // breaking case:
                        // width is not added yet!
                        if self.maybe_update(&c, &mut nodes[n+1]) {
                            last = n+1;
                        }
                    }
                    
                    // add width now.
                    c.measure += s.measure(self.width);
                }
                &StreamItem::Linebreak(fill) => {
                    if fill {
                        if self.width > c.measure.stretch {
                            c.measure.stretch = self.width;
                            if self.width > c.measure.width {
                                c.measure.width = self.width;
                            }
                        }
                    }
                
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
        
        if width < m.shrink {
            return false;
        }
        
        let factor = if width == m.width {
            1.0
        } else {
            let delta = width - m.width; // d > 0 => stretch, d < 0 => shrink
            let diff = if delta >= 0. {
                m.stretch - m.width
            } else {
                m.width - m.shrink
            };
            delta / diff
        };
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
 
