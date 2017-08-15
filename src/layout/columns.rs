use layout::{Entry, StreamVec, FlexMeasure, Item};
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

pub struct ParagraphLayout<'o, O: Output + 'o> {
    items:      &'o [Entry<O>],
    nodes:      Vec<Option<LineBreak>>,
    width:      Length,
    last:       usize
}
pub struct ColumnLayout<'o, O: Output + 'o> {
    para:       ParagraphLayout<'o, O>,
    nodes_col:  Vec<Option<ColumnBreak>>,
    height:     Length
}
impl<'o, O: Output + 'o> Debug for ColumnLayout<'o, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ColumnLayout")
    }
}
impl<'o, O: Output + 'o> Debug for ParagraphLayout<'o, O> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParagraphLayout")
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

impl<'o, O: Output+Debug> ParagraphLayout<'o, O> {
    pub fn new(items: &'o [Entry<O>], width: Length) -> ParagraphLayout<'o, O> {
        let limit = items.len();
        let mut nodes = vec![None; limit+1];
        nodes[0] = Some(LineBreak::default());

        let mut layout = ParagraphLayout {
            nodes,
            items,
            width,
            last: 0
        };
        layout.run();
        layout
    }
    fn run(&mut self) {
        let mut last = 0;
        for start in 0 .. self.items.len() {
            match self.nodes[start] {
                Some(b) => {
                    last = self.complete_line(
                        start,
                        Context::new(start, b.score)
                    );
                },
                None => {}
            }
        }

        if self.nodes[last].is_none() {
            for i in 0 .. last {
                println!("{:3} {:?}", i, self.items[i]);
                if let Some(b) = self.nodes[i] {
                    println!("     {:?}", b);
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
                Some(line) if break_score <= line.score => line,
                _ => break_point
            });

            true
        } else {
            false
        }
    }
    pub fn lines<'l>(&'l self) -> Column<'l, 'o, O> {
        Column::new(0, self.last, self)
    }
}
impl<'a, O: Output+Debug> ColumnLayout<'a, O>  {
    pub fn new(items: &'a StreamVec<O>, width: Length, height: Length) -> ColumnLayout<'a, O> {
        let limit = items.len();
        let mut nodes = vec![None; limit+1];
        let mut nodes_col = vec![None; limit+1];
        nodes[0] = Some(LineBreak::default());
        nodes_col[0] = Some(ColumnBreak::default());

        let mut layout = ColumnLayout {
            para: ParagraphLayout {
                nodes,
                items,
                width,
                last: 0
            },
            nodes_col,
            height,
        };
        layout.run();
        layout
    }
    pub fn columns<'l>(&'l self) -> Columns<'l, 'a, O> {
        Columns::new(self)
    }
    fn run(&mut self) {
        let mut last = 0;
        for start in 0 .. self.para.items.len() {
            match self.para.nodes[start] {
                Some(b) => {
                    last = self.para.complete_line(
                        start,
                        Context::new(start, b.score)
                    );
                    self.compute_column(start, false);
                },
                None => {}
            }
        }
        self.compute_column(last, true);

        if self.nodes_col[last].is_none() {
            for i in 0 .. last {
                println!("{:3} {:?}", i, self.para.items[i]);
                if let Some(b) = self.para.nodes[i] {
                    println!("     {:?}", b);
                }
                if let Some(l) = self.nodes_col[i] {
                    println!("     {:?}", l);
                }
            }
        }

        self.para.last = last;
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
            let last_node = self.para.nodes[last].unwrap();
                        
            if last > 0 {
                match self.para.items[last-1] {
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
                
                height += last_node.height;

                if height > self.height {
                    break;
                }
            }

            if let Some(column) = self.nodes_col[last] {
                let mut score = column.score
                    + self.num_lines_penalty(num_lines_at_last_break)
                    + self.num_lines_penalty(num_lines_before_end);
                
                if !is_last {
                    score += self.fill_penalty(height);
                }
            
                match self.nodes_col[n] {
                    Some(column) if column.score > score => {},
                    _ => {
                        self.nodes_col[n] = Some(ColumnBreak {
                            prev: last,
                            score: score
                        });
                        
                        found = true;
                    }
                }
            }

            if last == 0 {
                break;
            }
            last = last_node.prev;
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
        let mut last = layout.para.last;
        while last > 0 {
            columns.push(last);
            last = layout.nodes_col[last].unwrap().prev;
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
        self.columns.pop().map(|last| Column::new(
            self.layout.nodes_col[last].unwrap().prev,
            last,
            &self.layout.para
        ))
    }
}

#[derive(Debug)]
pub struct Column<'l, 'o: 'l, O: Output + 'o> {
    lines:      Vec<usize>, // points to the end of each line
    layout:     &'l ParagraphLayout<'o, O>,
    y:          Length
}
impl<'l, 'o: 'l, O: Output + 'o> Column<'l, 'o, O> {
    fn new(first: usize, mut last: usize, layout: &'l ParagraphLayout<'o, O>) -> Self {
        let mut lines = Vec::new();
        while last > first {
            lines.push(last);
            last = layout.nodes[last].unwrap().prev;
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
            let b = self.layout.nodes[last].unwrap();
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
    layout:     &'l ParagraphLayout<'o, O>,
    pos:        usize,
    end:        usize,
    branches:   usize,
    measure:    FlexMeasure,
    line:       LineBreak
}

impl<'l, 'o: 'l, O: Output + 'o> Iterator for Line<'l, 'o, O> {
    type Item = (f32, Item<'o, O>);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.end {
            let pos = self.pos;
            self.pos += 1;

            match self.layout.items[pos] {
                Entry::Word(ref w) | Entry::Punctuation(ref w) => {
                    let x = self.measure.at(self.line.factor);
                    self.measure += O::measure_word(w, self.layout.width);
                    return Some((x, Item::Word(w)));
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
                Entry::Anchor(ref data) => return Some((
                    self.measure.at(self.line.factor),
                    Item::Anchor(&*data)
                )),
                _ => {}
            }
        }
        
        None
    }
}
