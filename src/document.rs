use std::sync::Arc;
use std::fs::File;
use std::iter::{Iterator, FromIterator, IntoIterator, DoubleEndedIterator};
use std::collections::HashMap;
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

#[derive(Debug)]
enum Element {
    Word(Arc<DictEntry>),
    Reference(Arc<DictEntry>)
}

#[derive(Debug)]
enum NodeType {
    Block {
        kind:   String,
        head:   Vec<Element>,
        first:  Option<Arc<Node>>
    },
    Leaf(Vec<Element>)
}

#[derive(Debug)]
struct Node {
    next:   Option<Arc<Node>>,
    //source: String
    e:      NodeType
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
            parser::Item::Reference(r) => Element::Reference(self.intern(r))
        }
    }
    
    fn block(&mut self, b: &parser::Block, next: Option<Arc<Node>>) -> Arc<Node> {
        let mut prev: Option<Arc<Node>> = None;
        for child in b.body.iter().rev() {
            prev = match child {
                &parser::Body::Leaf(ref items) => {
                    Some(Arc::new(Node {
                        next:   prev,
                        e:      NodeType::Leaf(items.iter().map(|i| self.element_from_item(i)).collect())
                    }))
                },
                &parser::Body::Block(ref b) => Some(self.block(b, prev))
            }
        }           
        
        Arc::new(Node {
            next: next,
            e: NodeType::Block {
                kind:   b.name.to_owned(),
                head:   b.header.iter().map(|i| self.element_from_item(i)).collect(),
                first:  prev
            }
        })
    }
    
}

#[test]
fn test_document() {
    use std::io::Read;
    use nom::IResult;
    
    let mut f = File::open("doc/reference.yarn").unwrap();
    let mut data: Vec<u8> = Vec::new();
    f.read_to_end(&mut data).unwrap();
    
    let mut doc = Document::new();
    let root = match parser::block(&data, 0) {
        IResult::Done(i, b) => {
            println!("block: {:?}", b);
            doc.block(&b, None)
        },
        _ => panic!()
    };
        
    println!("doc: {:?}", doc);
    println!("root: {:?}", root);
    
    panic!();
}
   