use std::sync::{Arc, Weak};
use std::fs::File;
use std::iter::{Iterator, FromIterator, IntoIterator, DoubleEndedIterator};
use std::collections::HashMap;
use std::ops::Deref;
use daggy::{Dag};
use parser;

#[derive(Debug, PartialEq, Eq)]
struct DictEntry {
    full: String
}
impl DictEntry {
    fn new(w: &str) -> DictEntry {
        DictEntry {
            full: w.to_owned()
        }
    }
}


#[derive(Debug)]
struct Document {
    dict: HashMap<String, Arc<DictEntry>>,
    //root: Option<Arc<Node>>
}

#[derive(Debug, Clone)]
enum Element {
    Word(Arc<DictEntry>),
    Reference(Arc<DictEntry>),
    Symbol(Arc<DictEntry>),
    Punctuation(Arc<DictEntry>)
}

#[derive(Debug)]
enum NodeType {
    Block {
        kind:       String,
        head:       Vec<Element>,
        childs:     Vec<Node>
    },
    List {
        childs:     Vec<Node>
    },
    Leaf {
        elements:   Vec<Element>
    }
}

#[derive(Debug)]
struct NodeData {
    //source: String
    content:    NodeType
}

#[derive(Debug, Clone)]
struct Node {
    arc:    Arc<NodeData>
}
impl Node {
    fn new(content: NodeType) -> Node {
        Node{ arc: Arc::new(
            NodeData{
                content: content
            })
        }
    }
}
impl Deref for Node {
    type Target = NodeData;
    
    fn deref(&self) -> &NodeData {
        self.arc.deref()
    }
}

struct NodeIterator {
    // the stack contains the nodes that are about to be processed next.
    stack:  Vec<(Node, usize)>
}
impl Node {
    fn iter(&self) -> NodeIterator {
        NodeIterator {stack: vec![(self.clone(), 0)]}
    }
}
 
impl Iterator for NodeIterator {
    type Item = Element;
    
    fn next(&mut self) -> Option<Element> {
        while let Some((node, pos)) = self.stack.pop() {
            match node.content {
                NodeType::Block{ childs: ref c @ _, .. }
              | NodeType::List{ childs: ref c @ _, .. } 
                if c.len() > pos =>
                {
                    self.stack.push((node.clone(), pos+1)); // push back
                    self.stack.push((c[pos].clone(), 0));
                },
                
                NodeType::Leaf{ elements: ref e @ _}
                if e.len() > pos =>
                {
                    self.stack.push((node.clone(), pos+1));
                    return Some(e[pos].clone());
                },
                
                _ => {}
            }
        }
        None
    }
}

impl Document {
    fn new() -> Document {
        Document {
            dict: HashMap::new()
        }
    }
    
    fn intern(&mut self, w: &str) -> Arc<DictEntry> {
        match self.dict.get(w) {
            Some(e) => return e.clone(),
            None => ()
        }
        
        let e = Arc::new(DictEntry::new(w));
        let c = e.clone();
        self.dict.insert(w.to_owned(), e);
        return c;
    }
    
    fn element_from_item(&mut self, item: &parser::Item) -> Element {
        match *item {
            parser::Item::Word(w) => Element::Word(self.intern(w)),
            parser::Item::Reference(r) => Element::Reference(self.intern(r)),
            parser::Item::Symbol(s) => Element::Symbol(self.intern(s)),
            parser::Item::Punctuation(p) => Element::Punctuation(self.intern(p))
        }
    }
    
    fn list(&mut self, entries: &Vec<Vec<parser::Item>>) -> Node {
        Node::new(
            NodeType::List {
                childs: entries.iter().map(|e| self.leaf(e)).collect()
            }
        )
    }
    
    fn leaf(&mut self, items: &Vec<parser::Item>) -> Node {
        Node::new(
            NodeType::Leaf {
                elements: items.iter().map(|i| self.element_from_item(&i)).collect()
            }
        )
    }
    
    fn block(&mut self, b: &parser::Block) -> Node {
        Node::new(
            NodeType::Block {
                kind:   b.name.to_owned(),
                head:   b.header.iter().map(|i| self.element_from_item(&i)).collect(),
                childs: b.body.iter().map(|child| match child {
                    &parser::Body::Block(ref block) => self.block(block),
                    &parser::Body::List(ref entries) => self.list(entries),
                    &parser::Body::Leaf(ref items) => self.leaf(items)
                }).collect()
            }
        )
    }
}

#[test]
fn test_document() {
    use std::io::Read;
    use nom::IResult;
    
    let mut f = File::open("doc/reference.yarn").unwrap();
    let mut data = String::new();
    f.read_to_string(&mut data).unwrap();
    
    let mut doc = Document::new();
    let root = match parser::block(&data, 0) {
        IResult::Done(i, b) => {
            let root = doc.block(&b);
            if i.len() > 0 {
                println!("remaining:\n{:?}", i);
                panic!("not all input parsed");
            }
            root
        },
        _ => panic!()
    };
}
   