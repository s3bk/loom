use rmp;
use rmp_serialize;
use std::collections::HashMap;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std;
use woot::{IncrementalStamper};
use document::{Node, NodeP};
use layout::Writer;
use environment::{LocalEnv, GraphChain, LayoutChain, prepare_graph};
use futures::Future;
use super::LoomError;

pub use yaio::{self, AioError, File};
pub use rmp_serialize::{Encoder, Decoder};

pub type Result<T> = std::result::Result<T, AioError>;
pub type TypeId = u16;
pub type DataSize = u32;
pub type Stamp = (u32, u32);

pub struct IoCreate {
    stamp:  Stamp,
    io_ref: Io
}
impl IoCreate {
    pub fn submit(mut self, data: &[u8]) {
        // This is not Send -> submit can't be called twice at the same time
        self.io_ref.borrow_mut().add_data(self.stamp, data);
    }
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }
}

#[derive(Clone)]
pub struct Io {
    io:     Rc<RefCell<IoMachine>>
}
impl Io {
    fn borrow_mut(&self) -> RefMut<IoMachine> {
        self.io.borrow_mut()
    }
        
    pub fn load_yarn(&self, yarn: File) -> Box<Future<Item=Yarn, Error=LoomError>>
    {
        use blocks::Module;

        let io = self.clone();
        
        box yarn.read()
        .map_err(|e| e.into())
        .and_then(move |data| {
            let io = io;
            let string = String::from_utf8(data).expect("invalid utf8");
            let env = prepare_graph(&io);
            
            // the lifetime of io.clone() ensures no borrow exists when the function
            // returns from this call
            Module::parse(&io, env.clone(), string)
            .and_then(move |root: NodeP| {
                let io = io;
                println!("parsing complete");
                // thus this call can not fail
                io.borrow_mut().insert_node(root.clone());
                Ok(Yarn {
                    root:   root,
                    env:    env.take()
                })
            })
        })
    }
    
    pub fn create(&self) -> IoCreate {
        IoCreate {
            stamp:  self.borrow_mut().stamp(),
            io_ref: self.clone()
        }
    }
    
    pub fn fetch(&self, url: &str) -> yaio::ReadFuture {
        yaio::read_url(url)
    }
    
    pub fn base_dir(&self) -> yaio::Directory {
        yaio::default_directory()
    }
}

pub struct Yarn {
    root:   NodeP,
    env:    LocalEnv
}
impl Yarn {
    pub fn layout<W: Writer>(&self, w: &mut W) {
        self.root.layout(LayoutChain::root(&self.env), w)
    }
}

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
    nodes:      HashMap<Stamp, NodeP>,
    stamper:    IncrementalStamper<u32, u32>,
    typelist:   Vec<NodeType>,
}
impl IoMachine {
    fn add_data(&self, _stamp: Stamp, _data: &[u8]) {
    }
    fn stamp(&mut self) -> Stamp {
        self.stamper.stamp()
    }
    
    pub fn flush(&mut self) {
        //self.encoder.flush();
    }
    
    /// storage: Some(path) to store the document
    ///          None to throw it away.
    pub fn new() -> IoMachine {
        
        IoMachine {
            nodes:      HashMap::new(),
            stamper:    IncrementalStamper::init_random(),
            typelist:   vec![],
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
            self.nodes.insert(id, n);
        }
    }
    
    pub fn to_ref(self) -> Io {
        Io {
            io: Rc::new(RefCell::new(self))
        }
    }
}
