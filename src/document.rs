use std::rc::Rc;
use std::fs::File;
use std::iter::{Iterator, FromIterator, IntoIterator};
use radix_trie::{Trie, TrieKey};
use daggy::{Dag};
use nom::{FileProducer, Producer};
use parser::{LineConsumer};

#[derive(Debug, PartialEq, Eq)]
struct Word {
    full: String
}

#[derive(Debug)]
struct Block {
    name: String
}
impl Block {
    fn from_input(p: &mut FileProducer) -> Option<Block> {
        let mut consumer = LineConsumer::new();
        let c = &mut consumer;
        println!("first: {:?}", p.apply(c));
        println!("second: {:?}", p.apply(c));
        
        /*
        while let Some(l) = p.apply(c) {
            println!("loop: {:?}", l);
        }*/
        Some(Block { name: "foo".to_owned() })
    }
}
/*impl Consumer for Block {

}*/

#[derive(Debug)]
struct Document {
    dict: Trie<String, Rc<Word>>,
    root: Node
}

#[derive(Debug)]
enum Node {
    Word(Word),
    Block(Block)
}

impl Document {
    fn read_yarn(p: &mut FileProducer) -> Option<Node> {
        Block::from_input(p).map(|b| Node::Block(b))
    }
}

#[test]
fn test_document() {
    let mut p = FileProducer::new("doc/reference.yarn", 1024).unwrap();
    Document::read_yarn(&mut p);
}
   