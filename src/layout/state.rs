enum Hint {
    Glyph(char),
    LineStart,
    LineEnd,
    PageStart
}

struct LayoutState {
    // measure the width when breaking here
    real:   FlexMeasure,
    
    // 
    imag:   FlexMeasure,
    hint:   Hint,
    score:  f32
}

trait Item {
    fn apply(&self, state: LayoutState) -> LayoutState;
}



struct Node {
    state:  LayoutState,
    
    // score of this possible break
    score:  f32,
    
    // index into input items
    index:  usize
}

enum Element {
    Item(Item),
    BranchEntry(usize),
    BranchExit(usize)
}

struct LayoutSolver {
    nodes:  Vec<Node>,
    
}
struct LayoutInfo {
    width: Length
}
impl LayoutSolver {
    fn run() {
        // initial
        let mut state = LayoutState {
            real: 0.
            imag: 0.,
            hint: PageStart
        }
        let mut start = 0;
        loop {
            // grab current node
            let ref item = self.nodes[node.index];
        }
    }

    // find paths starting with the given state
    fn explore(mut node_index: usize) {
        let mut node = self.nodes[node_index];
        loop {
            // appy the item to the state
            state = item.apply(state);
            // Increment the index into the state vector
            node_index += 1;
            
            // The item generates possible break points.
            // It is important that this function always
            // returns the same value for a given entry.
            // Otherwise different runs might get inconsistent indices
            // into the state vector, causing undefined results.
            if let Some(score) = item.score() {
                node = Node {
                    state:  state,
                    index:  node.index + 1,
                    score:  node.score + score
                };
                
                match self.nodes.get(node_index) {
                    None => node_index.push(node);
                    Some(other) if other.score < node.score => self.nodes[node_index];
                }
            }
        }
    } 
}

struct NonBreakingSpace {
    measure:    FlexMeasure
}
impl Item for NonBreakingSpace {
    fn apply(&self, state: LayoutState) -> LayoutState {
        state.imag += self.measure;
        state
    }
}

struct BreakingSpace {
    measure:    FlexMeasure
}
impl Item for NonBreakingSpace {
    fn apply(&self, state: LayoutState) -> LayoutState {
        state.imag += self.measure;
        state
    }
    fn score(&self, info: LayoutInfo) {
        Some( info.width)
    }
}

struct Word<O> {
    w:  O::Word
}
impl Item for Word {
    fn apply(&self, state: LayoutState) -> LayoutState {
        LayoutState {
            real:   state.real + kerning(state.hint, self.w) + state.imag,
            imag:   0.,
            hint:   self.w
        }
    }

}

struct Newline {}
impl Item for Newline {
    fn apply(&self, _: LayoutState) -> LayoutState {
        LayoutState {
            real:   0.,
            imag:   0.,
            hint:   Hint::LineStart
        }
    }
}

struct HFill {}
impl Item for HFill {
    fn apply(&self, _: LayoutState) -> LayoutState {
        LayoutState {
            real:   0.,
            imag:   0.,
            hint:   Hint::LineStart
        }
    }
}

struct Hyphen {}
impl Item for Hyphen {
    fn apply(&self, state: LayoutState) -> LayoutState {
        LayoutState {
            real:   state.real,
            imag:   state.imag + kerning(state.hint, Glyph('-'))
            hint:   Glyph('-')
        }
    }
}
