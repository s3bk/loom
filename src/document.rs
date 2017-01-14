use std::iter::Iterator;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use layout::{Atom, Writer};
use environment::{LocalEnv, GraphChain};
use io::{Stamp, IoRef};
use woot::WString;
use inlinable_string::InlinableString;

/// The Document is a Directed Acyclic Graph.
///
/// One consequece is the possibility to use the same Node in more than
/// one place. Also moving parts of the document is not more than changing
/// a few references.

/// Everything is an Object -> is a Pointer -> is an ID
/// Since this is Rust, there will be a Trait requirement

pub type NodeP = Ptr<Node>;
pub type NodeListP = Ptr<NodeList<NodeP>>;

pub trait Node {
    /// when building the graph, this method is called
    /// to add child-nodes to the index
    fn childs(&self, &mut Vec<NodeP>) {}
    
    /// linearize the node
    //fn encode(&self, e: &mut Encoder);
    
    /// one or more child nodes were modified
    fn modified(&self) {}
    
    /// compute layout graph
    fn layout(&self, env: GraphChain, w: &mut Writer);
    
    /// ?
    fn add_ref(&self, _: &Rc<Node>) {}
    
    fn env(&self) -> Option<&LocalEnv> {
        None
    }
}

pub struct Ptr<N: ?Sized + Node> {
    rc: Rc<N>,
    //references: LinkedList<Ptr<N>>
}
impl<N: ?Sized> Node for Ptr<N> where N: Node {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.rc.childs(out)
    }
    fn modified(&self) {
        self.rc.modified()
    }
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        self.rc.layout(env, w)
    }
}
impl<N> Ptr<N> where N: Node {
    pub fn new(n: N) -> Ptr<N> {
        Ptr {
            rc: Rc::new(n),
            //references: LinkedList::new()
        }
    }
}
impl<N> From<Ptr<N>> for Ptr<Node> where N: Node + Sized + 'static {
    fn from(n: Ptr<N>) -> Ptr<Node> {
        Ptr {
            rc: n.rc as Rc<Node>
        }
    }
}

impl<N: ?Sized> Deref for Ptr<N> where N: Node {
    type Target = N;
    
    fn deref(&self) -> &N {
        self.rc.deref()
    }
}
impl<N> Ptr<N> where N: Node {
    pub fn get_mut(&mut self) -> Option<&mut N> {
        Rc::get_mut(&mut self.rc)
    }
}
impl<N: ?Sized> Clone for Ptr<N> where N: Node {
    fn clone(&self) -> Ptr<N> {
        Ptr {
            rc: self.rc.clone()
        }
    }
}

pub enum Placeholder {
    Body,
    Argument(usize),
    Arguments,
    Unknown(String)
}
impl Node for Placeholder {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        let fields = env.fields().unwrap();
        let n: Option<NodeP> = match self {
            &Placeholder::Body => fields.body().map(|n| n.into()),
            &Placeholder::Argument(i) => fields.args()
                .and_then(|n| n.iter().nth(i).cloned()),
            &Placeholder::Arguments => fields.args().map(|n| n.into()),
            _ => None
        };
        n.map(|n| n.layout(env.with_fields(fields.parent()), w))
        .unwrap_or_else(|| {
            println!("no macro set");
            match self {
                &Placeholder::Body => w.word(Atom::normal("$body")),
                &Placeholder::Argument(n) => w.word(Atom::normal(&format!("${}", n))),
                &Placeholder::Arguments => w.word(Atom::normal("$args")),
                &Placeholder::Unknown(ref s) => w.word(Atom::normal(&format!("${}", s))),
            }
        });
    }
}

pub struct Ref {
    name: String,
    target: RefCell<Option<NodeP>>
}
impl Ref {
    pub fn new(name: String) -> Ref {
        Ref {
            name:   name,
            target: RefCell::new(None)
        }
    }
    pub fn resolve(&mut self, env: GraphChain) {
        *self.target.borrow_mut() = env.get_target(&self.name).cloned();
    }
    pub fn get(&self) -> Option<NodeP> {
        match *self.target.borrow() {
            Some(ref n) => Some(n.clone()),
            None => None
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct GroupRef {
    key: (InlinableString, InlinableString),
    target: RefCell<Option<NodeP>>
}
impl GroupRef {
    pub fn new(opening: &str, closing: &str) -> GroupRef {
        GroupRef {
            key:    (opening.into(), closing.into()),
            target: RefCell::new(None)
        }
    }
    pub fn resolve(&mut self, env: GraphChain) {
        *self.target.borrow_mut() = env.get_group(&self.key).cloned();
    }
    pub fn get(&self) -> Option<NodeP> {
        match *self.target.borrow() {
            Some(ref n) => Some(n.clone()),
            None => None
        }
    }
    pub fn key(&self) -> &(InlinableString, InlinableString) {
        &self.key
    }
}

pub struct NodeList<T: Sized + Node + Clone> {
    ws: WString<T, Stamp>
}
impl<T> NodeList<T> where T: Node + Clone {
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a T> {
        self.ws.iter()
    }
    pub fn from<I>(io: IoRef, iter: I) -> NodeList<T>
    where I: Iterator<Item=T> {
        let mut ws = WString::new();
        
        for (n, item) in iter.enumerate() {
            let job = io.create();
            let op = ws.ins(n, item, job.stamp());
            //job.submit(op);
            
            //emit();
        }
        
        NodeList {
            ws: ws
        }
    }
    pub fn size(&self) -> usize {
        self.ws.len()
    }
}
impl<T> Node for NodeList<T> where T: Node + Sized + Clone + Into<NodeP> {
    fn childs(&self, out: &mut Vec<NodeP>) {
        for n in self.ws.iter() {
            out.push(n.clone().into());
        }
    }
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        for n in self.iter() {
            n.layout(env, w);
        }
    }
}
