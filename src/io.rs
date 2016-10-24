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
use document::{Node, NodeP};
use layout::TokenStream;
use environment::{Environment, prepare_environment, LocalEnv};

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
    encoder:    lz4::Encoder<DataOut>,
    nodes:      HashMap<Stamp, NodeP>,
    stamper:    RefCell<IncrementalStamper<u32, u32>>,
    typelist:   Vec<NodeType>,
    active:     Option<(NodeP, LocalEnv)>
}
pub struct IoCreate<'a> {
    stamp:  Stamp,
    io_ref: IoRef<'a>
}
impl<'a> IoCreate<'a> {
    pub fn submit(mut self, data: &[u8]) {
        // This is not Send -> submit can't be called twice at the same time
        self.io_ref.io.add_data(self.stamp, data);
    }
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }
}

#[derive(Clone)]
pub struct IoRef<'a> {
    io:     &'a IoMachine
}
impl<'a> IoRef<'a> {
    pub fn create(&self) -> IoCreate {
        IoCreate {
            stamp:  self.io.stamp(),
            io_ref: self.clone()
        }
    }
    pub fn new(io: &IoMachine) -> IoRef {
        IoRef{
            io: io
        }
    }
}

struct DataOut {
    file:   Option<File>
}
impl io::Write for DataOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use std::char;
        
        println!("write: {}", {
            let s: String = buf.iter()
                .map(|&b| char::from_u32(b as u32 + 0x2800).unwrap())
                .collect();
            s
        });
        
        if let Some(ref mut f) = self.file {
            f.write(buf)
        } else {
            Ok(buf.len())
        }
    }
    
    fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut f) = self.file {
            f.flush()
        } else {
            Ok(())
        }
    }
}

impl IoMachine {
    fn add_data(&self, stamp: Stamp, data: &[u8]) {
    }
    fn stamp(&self) -> Stamp {
        self.stamper.borrow_mut().stamp()
    }
    
    pub fn flush(&mut self) {
        //self.encoder.flush();
    }
    
    /// storage: Some(path) to store the document
    ///          None to throw it away.
    pub fn new(storage: Option<&Path>) -> IoMachine {
        use lz4::liblz4::{BlockMode, BlockSize};
        
        let file = storage.map(|p| File::open(p).unwrap());
        let mut data_out = DataOut {
            file:   file
        };
        
        let encoder = lz4::EncoderBuilder::new()
        .block_mode(BlockMode::Linked)
        .block_size(BlockSize::Max1MB)
        .level(16)
        .build(data_out)
        .expect("failed to create lz4 encoder");
        
        IoMachine {
            encoder:    encoder,
            nodes:      HashMap::new(),
            stamper:    RefCell::new(IncrementalStamper::init_random()),
            typelist:   vec![],
            active:     None
        }
    }
    
    pub fn insert_node(&mut self, node: NodeP) {
        // Nodes may have strange links. Avoid recursion!
        let mut queue = vec![node];
        
        while let Some(n) = queue.pop() {
            // add childs to quue
            n.childs(&mut queue);
            
            // make up an ID
            let id = self.stamper.get_mut().stamp();
            
            
            // push into queue
            //self.queue_out.push(Shared{ id: id, node: n });
            
            // store object (consumes it)
            self.nodes.insert(id, n);
        }
    }

    pub fn load_yarn(&mut self, yarn: &Path) {
        use blocks::RootNode;
        use std::io::Read;

        println!("load yarn: {:?}", yarn);
        
        let mut env = prepare_environment();

        let mut data = String::new();
        File::open(yarn).expect("could not open file")
        .read_to_string(&mut data).expect("could not read from file");
        
        // the lifetime of io.clone() ensures no borrow exists when the function
        // returns from this call
        let root = RootNode::parse(IoRef::new(self), Environment::root(&env), &data);
        println!("parsing complete");
        // thus this call can not fail
        self.insert_node(root.clone());
        
        self.active = Some((root, env));
    }
    
    pub fn layout(&self, s: &mut TokenStream) {
        match self.active {
            Some((ref root, ref env)) =>
                root.layout(Environment::root(env), s),
            None => {}
        }
    }
}
