use layout::{Entry, StreamVec, FlexMeasure, Surface};
use output::{Output};
use num::Zero;
//use layout::style::{Style};

#[derive(Copy, Clone, Debug, Default)]
struct LineBreak {
    prev:   usize,
    path:   u64, // one bit for each branch taken (1) or not (0)
    factor: f32,
    score:  f32
}
pub struct ParagraphLayout<'a, O: Output + 'a> {
    items:      &'a StreamVec<O>,
    width:      f32,
    surface:    &'a mut O::Surface
}

struct LineContext {
    measure:    FlexMeasure,
    path:       u64, // one bit for each branch on this line
    begin:      usize, // begin of line or branch
    pos:        usize, // calculation starts here
    score:      f32, // score at pos
    branches:   u8 // number of branches so far (<= 64)
}

impl<'a, O: Output> ParagraphLayout<'a, O>  {
    pub fn new(items: &'a StreamVec<O>, surface: &'a mut O::Surface, )
     -> ParagraphLayout<'a, O>
    {
        ParagraphLayout {
            items:      items,
            width:      surface.primary(),
            surface:    surface
        }
    }
    
    pub fn run(&mut self) {
        use std::iter::repeat;
        
        let limit = self.items.len();
        let mut nodes: Vec<Option<LineBreak>> = repeat(None).take(limit+1).collect();
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
        /*
        for (n, node) in nodes.iter().take(limit).enumerate() {
            println!("{:4}  {:?}", n, node);
            println!("      {:?}", self.items[n]);
        }
        println!("{:4}  {:?}", limit, nodes[limit]);
        println!("last: {}", last);
        */
        if last == 0 {
            return;
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
        
        let mut y = 20.;
        for &(b, end) in steps.iter().rev() {
            let mut measure = FlexMeasure::zero();
            let mut pos = b.prev;
            let mut branches = 0;
            while pos < end {
                match self.items[pos] {
                    Entry::Word(ref w) | Entry::Punctuation(ref w) => {
                        let x = measure.at(b.factor);
                        measure += O::measure_word(w, self.width);
                        O::draw_word(self.surface, (x, y), w);
                    },
                    Entry::Space(_, s) => {
                        measure += s;
                    },
                    Entry::BranchEntry(len) => {
                        if b.path & (1<<branches) == 0 {
                            // not taken
                            pos += len;
                        }
                        branches += 1;
                    },
                    Entry::BranchExit(skip) => pos += skip,
                    Entry::Linebreak(_) => unreachable!(),
                    _ => {}
                }
                pos += 1;
            }
            
            y += measure.height;
        }
    }
    
    fn complete_line(&self, nodes: &mut Vec<Option<LineBreak>>, c: LineContext) -> usize {
        let mut last = c.begin;
        let mut c = c;
        
        while c.pos < self.items.len() {
            let n = c.pos;
            match self.items[n] {
                Entry::Word(ref w) | Entry::Punctuation(ref w) => {
                    c.measure += O::measure_word(w, self.width);
                },
                Entry::Space(breaking, s) => {
                    if breaking {
                        // breaking case:
                        // width is not added yet!
                        if self.maybe_update(&c, &mut nodes[n+1]) {
                            last = n+1;
                        }
                    }
                    
                    // add width now.
                    c.measure += s;
                }
                Entry::Linebreak(fill) => {
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
                Entry::BranchEntry(len) => {
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
                Entry::BranchExit(skip) => {
                    c.pos += skip;
                }
                _ => {}
            }
            
            if c.measure.shrink > self.width {
                break; // too full
            }
            
            c.pos += 1;
        }
        
        last
    }
    
    
    fn maybe_update(&self, c: &LineContext, node: &mut Option<LineBreak>) -> bool {
        if let Some(factor) = c.measure.factor(self.width) {
            let break_score = c.score - factor * factor;
            let break_point = LineBreak {
                prev:   c.begin,
                path:   c.path,
                factor: factor,
                score:  break_score
            };
            match *node {
                None => {
                    *node = Some(break_point);
                },
                Some(other) if break_score > other.score => {
                    *node = Some(break_point);
                },
                _ => {}
            }
            true
        } else {
            false
        }
    }
}
 
