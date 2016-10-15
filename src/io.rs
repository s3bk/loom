use rmp;
use rmp_serialize;
use rustc_serialize::Encodable;
use lz4;
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use std::collections::HashMap;
use std::rc::Rc;
use woot::{WString, WStringIter, IncrementalStamper, Key};
use pipe::{pipe, PipeReader, PipeWriter};
use document::{Node, NodeP, Children};
use layout::LayoutNode;
use environment::{Environment, prepare_environment};

pub use rmp_serialize::{Encoder, Decoder};

pub type Stamp = (u32, u32);
pub type TypeId = u16;
pub type DataSize = u32;

struct NodeType {
    name:   String,
    //decode: Box<Fn(&mut Decoder<Vec<u8>>) -> Result<Decoder::Error>>,
}

enum StreamItem {
    /// add a new node to the known nodes.
    /// This if the name is known, the appropriate entry is inserted into the
    /// IoMachine and the environment.
    /// Optionally it can be tried to load the name from a dynamic library.
    NodeDecl(String),
    CreateNode(TypeId, DataSize, Stamp),
    DeleteNode(Stamp),
    Op(Stamp, DataSize)
}
pub struct IoMachine {
    // we write everything to disk as well
    // uncompressed data to go into compressor
    encoder:    lz4::Encoder<PipeWriter>,
    file:       Option<File>,
    nodes:      HashMap<Stamp, Box<Node>>,
    changes:    PipeReader,
    stamper:    IncrementalStamper<u32, u32>,
    typelist:   Vec<NodeType>
}
impl IoMachine {
    
    pub fn create_batch<F>(&mut self, f: F) where F: Fn(fn() -> Stamp, fn(&Node)) {
        // prepare
        let &mut encoder = rmp_serialize::Encoder::new(self.encoder);
        f(self.stamper.stamp, |item: &Node| item.encode(&mut encoder));
        // finish
    }
    
    pub fn flush(&mut self) {
        self.encoder.flush();
    }
    
    /// storage: Some(path) to store the document
    ///          None to throw it away.
    pub fn create(storage: Option<&Path>) -> IoMachine {
        use lz4::liblz4::{BlockMode, BlockSize};
        use environment::prepare_environment;
        
        let (pipe_in, pipe_out) = pipe();
        let file = storage.map(|p| File::open(p).unwrap());
        
        let encoder = lz4::EncoderBuilder::new()
        .block_mode(BlockMode::Linked)
        .block_size(BlockSize::Max1MB)
        .level(16)
        .build(pipe_in)
        .expect("failed to create lz4 encoder");
        
        IoMachine {
            encoder:    encoder,
            file:       file,
            changes:    pipe_out,
            nodes:      HashMap::new(),
            stamper:    IncrementalStamper::init_random()
        }
    }
    
    pub fn load_yarn(&mut self, yarn: Path) {
        use blocks::RootNode;
    
        let mut env = Environment::new(self);
        prepare_environment(&mut env);
    
        let mut data = String::new();
        File::open(yarn).unwrap().read_to_string(&mut data).unwrap();
        let root = RootNode::parse(env, data);
        self.insert_node(root);
    }
    
    pub fn insert_node(&mut self, node: Rc<Node>) {
        // Nodes may have strange links. Avoid recursion!
        let mut queue = vec![node];
        
        while let Some(n) = queue.pop() {
            // make up an ID
            let id = self.stamper.stamp();
            
            // store object
            self.nodes.insert(id, node.clone());
            
            // push into queue
            //self.queue_out.push(Shared{ id: id, node: n });
            
            // add childs to quue
            queue.extend(n.childs());
        }
    }
}

