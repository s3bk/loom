use rmp;
use rmp_serialize;
use rustc_serialize::Encodable;
use lz4;
use std::fs::File;
use std::io;
use std::path::Path;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;
use woot::{WString, WStringIter, IncrementalStamper, Key};
use pipe::{pipe, PipeReader, PipeWriter};
use document::{Node, NodeP};
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
    nodes:      HashMap<Stamp, NodeP>,
    changes:    PipeReader,
    stamper:    IncrementalStamper<u32, u32>,
    typelist:   Vec<NodeType>
}
pub struct IoCreate<'a> {
    stamp:  Stamp,
    io:     RefCell<IoMachine>,
    marker: PhantomData<&'a IoRef<'a>>
}
impl<'a> IoCreate<'a> {
    pub fn submit(mut self, data: &[u8]) {
        // This is not Send -> submit can't be called twice at the same time
        self.io.borrow_mut().add_data(self.stamp, data);
    }
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }
}

pub struct IoRef<'a> {
    io:     RefCell<IoMachine>,
    marker: PhantomData<&'a IoRef<'a>>
}
impl<'a> IoRef<'a> {
    pub fn create(&self) -> IoCreate {
        IoCreate {
            stamp:  self.io.borrow_mut().stamp(),
            io:     self.io.clone(),
            marker: PhantomData
        }
    }
    pub fn new(io: IoMachine) -> IoRef<'static> {
        IoRef{
            io: RefCell::new(io)
        }
    }
    fn clone(&self) -> IoRef {
        IoRef{
            io:     self.io.clone(),
            marker: PhantomData(self)
        }
    }
}


impl IoMachine {
    fn add_data(&mut self, stamp: Stamp, data: &[u8]) {
    }
    pub fn stamp(&self) -> Stamp {
        self.stamper.stamp()
    }
    
    pub fn flush(&mut self) {
        //self.encoder.flush();
    }
    
    /// storage: Some(path) to store the document
    ///          None to throw it away.
    pub fn new(storage: Option<&Path>) -> IoMachine {
        use lz4::liblz4::{BlockMode, BlockSize};
        use environment::prepare_environment;
        
        let (pipe_out, pipe_in) = pipe();
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
            stamper:    IncrementalStamper::init_random(),
            typelist:   vec![]
        }
    }
    
    pub fn insert_node(&mut self, node: NodeP) {
        // Nodes may have strange links. Avoid recursion!
        let mut queue = vec![node];
        
        while let Some(n) = queue.pop() {
            // add childs to quue
            n.childs(&mut queue);
            
            // make up an ID
            let id = self.stamper.stamp();
            
            
            // push into queue
            //self.queue_out.push(Shared{ id: id, node: n });
            
            // store object (consumes it)
            self.nodes.insert(id, node);
        }
    }
}


pub fn load_yarn(mut io: IoRef, yarn: &Path) {
    use blocks::RootNode;
    use std::io::Read;

    let mut env = Environment::new();
    prepare_environment(&mut env);

    let mut data = String::new();
    File::open(yarn).expect("could not open file")
    .read_to_string(&mut data).expect("could not read from file");
    
    // the lifetime of io.clone() ensures no borrow exists when the function
    // returns from this call
    let root = RootNode::parse(io.clone(), &env, &data);
    
    // thus this call can not fail
    io.borrow_mut().insert_node(root);
}

#[test]
fn test_io() {
    let mut io = IoRef::new(IoMachine::new(None));
    

}
