use layout::{Entry, StreamVec, FlexMeasure};
use output::{Output};
use num::Zero;
//use layout::style::{Style};
use units::Length;
use std::fmt::{self, Debug};

#[derive(Copy, Clone, Debug, Default)]
struct LineBreak {
    prev:   usize, // index to previous line-break
    path:   u64, // one bit for each branch taken (1) or not (0)
    factor: f32,
    score:  f32,
    height: f32,
}

#[derive(Copy, Clone, Debug, Default)]
struct ColumnBreak {
    prev:   usize, // index to previous column-break
    score:  f32,
}
    
#[derive(Copy, Clone, Debug, Default)]
struct Break {
    line:   LineBreak,
    column: Option<ColumnBreak>
}

pub struct ColumnLayout<'o, O: Output + 'o> {
    items:      &'o StreamVec<O>,
    nodes:      Vec<Option<Break>>,
    width:      Length,
    height:     Length,
    last:       usize
}
impl<'o, O: Output + 'o> Debug for ColumnLayout<'o, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ColumnLayout")
    }
}

struct Context {
    measure:    FlexMeasure,
    path:       u64,    // one bit for each branch on this line
    begin:      usize,  // begin of line or branch
    pos:        usize,  // calculation starts here
    score:      f32,    // score at pos
    branches:   u8,     // number of branches so far (<= 64)
    punctuaton: FlexMeasure
}
impl Context {
    fn new(start: usize, score: f32) -> Context {
        Context {
            measure:    FlexMeasure::zero(),
            path:       0,
            begin:      start,
            pos:        start,
            branches:   0,
            score:      score,
            punctuaton: FlexMeasure::zero()
        }
    }
    fn add_word(&mut self, measure: FlexMeasure) {
        self.measure += self.punctuaton + measure;
        self.punctuaton = FlexMeasure::zero();
    }
    fn add_punctuation(&mut self, measure: FlexMeasure) {
        self.punctuaton = measure;
    }
    fn line(&self) -> FlexMeasure {
        self.measure + self.punctuaton * 0.5
    }
    fn fill(&mut self, width: Length) {
        self.measure = self.line();
        self.measure.extend(width);
        self.punctuaton = FlexMeasure::zero();
    }
}

impl<'a, O: Output+Debug> ColumnLayout<'a, O>  {
    pub fn new(items: &'a StreamVec<O>, width: Length, height: Length) -> ColumnLayout<'a, O> {
        let limit = items.len();
        let mut nodes: Vec<Option<Break>> = vec![None; limit+1];
        nodes[0] = Some(Break {
            line: LineBreak::default(),
            column: Some(ColumnBreak::default())
        });

        let mut layout = ColumnLayout {
            nodes,
            items,
            width,
            height,
            last: 0
        };
        layout.run();
        layout
    }
    pub fn columns<'l>(&'l self) -> Columns<'l, 'a, O> {
        Columns::new(self)
    }
    fn run(&mut self) {
        let mut last = 0;
        for start in 0 .. self.items.len() {
            match self.nodes[start] {
                Some(b) => {
                    last = self.complete_line(
                        start,
                        Context::new(start, b.line.score)
                    );
                    self.compute_column(start, false);
                },
                None => {}
            }
        }
        self.compute_column(last, true);

        if self.nodes[last].unwrap().column.is_none() {
            for i in 0 .. last {
                println!("{:3} {:?}", i, self.items[i]);
                if let Some(b) = self.nodes[i] {
                    println!("     {:?}", b.line);
                    if let Some(l) = b.column {
                        println!("     {:?}", l);
                    }
                }
            }
        }

        self.last = last;
    }
    
    fn complete_line(&mut self, start: usize, mut c: Context) -> usize {
        let mut last = c.begin;
        
        while c.pos < self.items.len() {
            let n = c.pos;
            match self.items[n] {
                Entry::Word(ref w) => c.add_word(O::measure_word(w, self.width)),
                Entry::Punctuation(ref w) => c.add_punctuation(O::measure_word(w, self.width)),
                Entry::Space(breaking, s) => {
                    if breaking {
                        // breaking case:
                        // width is not added yet!
                        if self.maybe_update(&c, n+1) {
                            last = n+1;
                        }
                    }
                    
                    // add width now.
                    c.measure += s;
                }
                Entry::Linebreak(fill) => {
                    if fill {
                        c.fill(self.width);
                    }
                    
                    if self.maybe_update(&c, n+1) {
                        last = n+1;
                    }
                    break;
                },
                Entry::BranchEntry(len) => {
                    // b
                    let b_last = self.complete_line(
                        start,
                        Context {
                            pos:        n + 1,
                            path:       c.path | (1 << c.branches),
                            branches:   c.branches + 1,
                            ..          c
                        }
                    );
                    if b_last > last {
                        last = b_last;
                    }
                    
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

    fn maybe_update(&mut self, c: &Context, n: usize) -> bool {
        if let Some(factor) = c.line().factor(self.width) {
            let break_score = c.score - factor * factor;
            let break_point = LineBreak {
                prev:   c.begin,
                path:   c.path,
                factor: factor,
                score:  break_score,
                height: c.measure.height
            };
            self.nodes[n] = Some(match self.nodes[n] {
                Some(Break { line, column }) => Break {
                    line:   if break_score > line.score { break_point } else { line },
                    column: column
                },
                None => Break {
                    line:   break_point,
                    column: None
                }
            });

            true
        } else {
            false
        }
    }

    fn num_lines_penalty(&self, n: usize) -> f32 {
        match n {
            1 => -20.0,
            2 => -2.0,
            _ => 0.0
        }
    }
    fn fill_penalty(&self, fill: Length) -> f32 {
        -10.0 * (self.height - fill) / self.height
    }

    fn compute_column(&mut self, n: usize, is_last: bool) -> bool {
        //                                        measure:
        let mut num_lines_before_end = 0;      // - lines before the break; reset between paragraphs
        let mut num_lines_at_last_break = 0;   // - lines after the previous break; count until the last paragraph starts
        let mut is_last_paragraph = true;
        let mut height = 0.0;
        let mut last = n;
        let mut found = false;
        
        loop {
            let last_node = self.nodes[last].unwrap();
                        
            if last > 0 {
                match self.items[last-1] {
                    Entry::Linebreak(_) => {
                        is_last_paragraph = false;
                        num_lines_before_end = 0;
                    },
                    Entry::Space { .. } => {
                        num_lines_before_end += 1;

                        if is_last_paragraph {
                            num_lines_at_last_break += 1;
                        }
                    }
                    ref e => panic!("found: {:?}", e)
                }
                
                height += last_node.line.height;

                if height > self.height {
                    break;
                }
            }

            if let Some(column) = last_node.column {
                let mut score = column.score
                    + self.num_lines_penalty(num_lines_at_last_break)
                    + self.num_lines_penalty(num_lines_before_end);
                
                if !is_last {
                    score += self.fill_penalty(height);
                }
            
                match self.nodes[n].unwrap() {
                    Break { line, column } => {
                        self.nodes[n] = Some(Break {
                            line: line,
                            column: Some(match column {
                                Some(column) if column.score > score => column,
                                _ => ColumnBreak {
                                    prev: last,
                                    score: score
                                }
                            })
                        });

                        found = true;
                    }
                }
            }

            if last == 0 {
                break;
            }
            last = last_node.line.prev;
        }
        
        found
    }
}

#[derive(Debug)]
pub struct Columns<'l, 'o: 'l, O: Output + 'o> {
    layout:     &'l ColumnLayout<'o, O>,
    columns:    Vec<usize>
}
impl<'l, 'o: 'l, O: Output + 'o> Columns<'l, 'o, O> {
    fn new(layout: &'l ColumnLayout<'o, O>) -> Self {
        let mut columns = Vec::new();
        let mut last = layout.last;
        while last > 0 {
            columns.push(last);
            last = layout.nodes[last].unwrap().column.unwrap().prev;
        }
        Columns {
            layout: layout,
            columns: columns
        }
    }
}
impl<'l, 'o: 'l, O: Output + 'o> Iterator for Columns<'l, 'o, O> {
    type Item = Column<'l, 'o, O>;

    fn next(&mut self) -> Option<Self::Item> {
        self.columns.pop().map(|last| Column::new(last, self.layout))
    }
}

#[derive(Debug)]
pub struct Column<'l, 'o: 'l, O: Output + 'o> {
    lines:      Vec<usize>, // points to the end of each line
    layout:     &'l ColumnLayout<'o, O>,
    y:          Length
}
impl<'l, 'o: 'l, O: Output + 'o> Column<'l, 'o, O> {
    fn new(mut last: usize, layout: &'l ColumnLayout<'o, O>) -> Self {
        let first = layout.nodes[last].unwrap().column.unwrap().prev;
        
        let mut lines = Vec::new();
        while last > first {
            lines.push(last);
            last = layout.nodes[last].unwrap().line.prev;
        }
        
        Column {
            lines: lines,
            layout: layout,
            y: 0.0
        }
    }
}
impl<'l, 'o: 'l, O: Output + 'o> Iterator for Column<'l, 'o, O> {
    type Item = (Length, Line<'l, 'o, O>);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.lines.pop().map(|last| {
            let b = self.layout.nodes[last].unwrap().line;
            self.y += b.height;
            
            (self.y, Line {
                layout:   self.layout,
                pos:      b.prev,
                branches: 0,
                measure:  FlexMeasure::zero(),
                line:     b,
                end:      last-1
            })
        })
    }
}

#[derive(Debug)]
pub struct Line<'l, 'o: 'l, O: Output + 'o> {
    layout:     &'l ColumnLayout<'o, O>,
    pos:        usize,
    end:        usize,
    branches:   usize,
    measure:    FlexMeasure,
    line:       LineBreak
}

impl<'l, 'o: 'l, O: Output + 'o> Iterator for Line<'l, 'o, O> {
    type Item = (f32, &'o O::Word);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.end {
            let pos = self.pos;
            self.pos += 1;

            match self.layout.items[pos] {
                Entry::Word(ref w) | Entry::Punctuation(ref w) => {
                    let x = self.measure.at(self.line.factor);
                    self.measure += O::measure_word(w, self.layout.width);
                    return Some((x, w));
                },
                Entry::Space(_, s) => {
                    self.measure += s;
                },
                Entry::BranchEntry(len) => {
                    if self.line.path & (1<<self.branches) == 0 {
                        // not taken
                        self.pos += len;
                    }
                    self.branches += 1;
                },
                Entry::BranchExit(skip) => self.pos += skip,
                Entry::Linebreak(_) => unreachable!(),
                _ => {}
            }
        }
        
        None
    }
}
