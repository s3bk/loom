#![allow(dead_code)]

//use rmp;
//use rmp_serialize;
use std::collections::HashMap;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::fmt;
use woot::{IncrementalStamper};
use document::{Node, NodeP};
use layout::Writer;
use environment::{LocalEnv, LayoutChain, prepare_graph};
use futures::Future;
use wheel::prelude::*;
use config::Config;
use super::LoomError;

pub type TypeId = u16;
pub type DataSize = u32;
pub type Stamp = (u32, u32);
pub type Data = <File as AsyncRead>::Buffer;

pub struct IoCreate {
    stamp:  Stamp,
    io_ref: Io
}
impl IoCreate {
    pub fn submit(self, data: &[u8]) {
        // This is not Send -> submit can't be called twice at the same time
        self.io_ref.borrow_mut().add_data(self.stamp, data);
    }
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }
}

#[derive(Clone)]
pub struct Io {
        io:     Rc<RefCell<IoMachine>>,
    pub log:    Log
}
impl Io {
    fn borrow_mut(&self) -> RefMut<IoMachine> {
        self.io.borrow_mut()
    }
    
    pub fn yarn(&self, yarn: String) -> Box<Future<Item=Yarn, Error=LoomError>> {
        use blocks::Module;
        let env = prepare_graph(self);
            
        let io = self.clone();
        // the lifetime of io.clone() ensures no borrow exists when the function
        // returns from this call
        box Module::parse(io.clone(), env.clone(), yarn)
        .and_then(move |root: NodeP| {
            let io = io;
            // thus this call can not fail
            io.borrow_mut().insert_node(root.clone());
            Ok(Yarn {
                root:   root,
                env:    env.take()
            })
        })
    }
    
    pub fn load_yarn(&self, yarn: File) -> Box<Future<Item=Yarn, Error=LoomError>>
    {

        let io = self.clone();
        
        trace!(self.log, "load_yarn");
        
        box read(yarn)
        .and_then(move |data| {
            let io = io;
            let string = String::from_utf8(data.to_vec()).expect("invalid utf8");
            io.yarn(string)
        })
    }
    
    pub fn create(&self) -> IoCreate {
        IoCreate {
            stamp:  self.borrow_mut().stamp(),
            io_ref: self.clone()
        }
    }
    
    pub fn config<F, O>(&self, f: F) -> O where F: FnOnce(&Config) -> O {
        f(&self.io.borrow().config)
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
impl fmt::Debug for Yarn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Yarn")
    }
}


struct NodeType {
    _name:   String,
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
    config:     Config,
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
    pub fn new(config: Config) -> IoMachine {
        
        IoMachine {
            nodes:      HashMap::new(),
            stamper:    IncrementalStamper::init_random(),
            typelist:   vec![],
            config:     config
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
            io:     Rc::new(RefCell::new(self)),
            log:    Log::root().branch()
        }
    }
}

pub fn open_dir(name: &str) -> Box<Future<Item=Directory, Error=LoomError>>
{
    box Directory::open(name)
    .map_err(|e| LoomError::DirectoryOpen(e))
}
pub fn open(dir: &Directory, name: &str) -> Box<Future<Item=File, Error=LoomError>>
{
    box dir.get_file(name)
    .map_err(|e| LoomError::DirectoryGetFile(e))
}
pub fn read(file: File) -> Box<Future<Item=Data, Error=LoomError>> {
    box file.read().map_err(|e| LoomError::FileRead(e))
}
pub fn open_read(dir: &Directory, name: &str) -> Box<Future<Item=Data, Error=LoomError>> {
    box dir.get_file(name)
    .map_err(|e| LoomError::DirectoryGetFile(e))
    .and_then(|file| file.read().map_err(|e| LoomError::FileRead(e)))
}
