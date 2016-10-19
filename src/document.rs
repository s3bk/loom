use std::iter::Iterator;
use std::ops::Deref;
use std::rc::Rc;
use std::fmt::Debug;
use std::cell::RefCell;
use layout::{LayoutNode};
use environment::Environment;
use io::{Stamp, IoRef};
use woot::{WString, WStringIter};
use slog::Logger;

/// The Document is a Directed Acyclic Graph.
///
/// One consequece is the possibility to use the same Node in more than
/// one place. Also moving parts of the document is not more than changing
/// a few references.

/// Everything is an Object -> is a Pointer -> is an ID
/// Since this is Rust, there will be a Trait requirement

pub type NodeP = P<Node>;
pub trait Node: Debug {    
    /// 
    fn childs(&self, &mut Vec<NodeP>) {}
    
    //fn encode(&self, e: &mut Encoder);
    
    // one or more child nodes were modified
    fn modified(&self) {}
    
    /// compute layout graph
    fn layout(&self, env: &Environment) -> LayoutNode;

    fn space(&self) -> (bool, bool) {
        (false, false)
    }
    fn add_ref(&self, _: &Rc<Node>) {}
}
#[derive(Debug)]
pub struct P<N: ?Sized + Node> {
    rc: Rc<N>,
    //references: LinkedList<P<N>>
}
impl<N: ?Sized> Node for P<N> where N: Node {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.rc.childs(out)
    }
    fn modified(&self) {
        self.rc.modified()
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        self.rc.layout(env)
    }
    fn space(&self) -> (bool, bool) {
        self.rc.space()
    }
}
impl<N> P<N> where N: Node {
    pub fn new(n: N) -> P<N> {
        P {
            rc: Rc::new(n),
            //references: LinkedList::new()
        }
    }
}
impl<N> From<P<N>> for P<Node> where N: Node + Sized + 'static {
    fn from(n: P<N>) -> P<Node> {
        P {
            rc: n.rc as Rc<Node>
        }
    }
}

impl<N> Deref for P<N> where N: Node {
    type Target = N;
    
    fn deref(&self) -> &N {
        self.rc.deref()
    }
}
impl<N> P<N> where N: Node {
    pub fn get_mut(&mut self) -> Option<&mut N> {
        Rc::get_mut(&mut self.rc)
    }
}
impl<N: ?Sized> Clone for P<N> where N: Node {
    fn clone(&self) -> P<N> {
        P {
            rc: self.rc.clone()
        }
    }
}

#[derive(Debug)]
pub struct Cached<N: Node> {
    cache: RefCell<Option<LayoutNode>>,
    pub inner: N
}
impl<N> Cached<N> where N: Node {
    pub fn new(node: N) -> Cached<N> {
        Cached {
            cache:  RefCell::new(None),
            inner:  node
        }
    }
}
impl<N> Node for Cached<N> where N: Node {
    fn layout(&self, env: &Environment) -> LayoutNode {
        let mut cache = self.cache.borrow_mut();
        if let Some(ref l) = *cache {
            return l.clone();
        }
        let l = self.inner.layout(env);
        *cache = Some(l.clone());
        l
    }
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.inner.childs(out)
    }
    fn modified(&self) {
        *self.cache.borrow_mut() = None;
    }
}

#[derive(Debug)]
pub enum Placeholder {
    Body,
    Argument(usize),
    Arguments,
    Unknown(String)
}
impl Node for Placeholder {
    fn layout(&self, env: &Environment) -> LayoutNode {
        use blocks::LeafBuilder;
        
        match env.get_macro() {
            Some(m) => m.placeholder_layout(env, self),
            None => {
                info!(env, "no macro set");
                let b = LeafBuilder::new(env);
                match self {
                    &Placeholder::Body => b.word("$body"),
                    &Placeholder::Argument(n) => b.word(&format!("${}", n)),
                    &Placeholder::Arguments => b.word("$args"),
                    &Placeholder::Unknown(ref s) => b.word(&format!("${}", s)),
                }.build()
            }
        }
    }
}

pub trait Macro: Node {
    fn placeholder_layout(&self, env: &Environment, p: &Placeholder) -> LayoutNode;
}


#[derive(Debug)]
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
    pub fn resolve(&mut self, env: &Environment) {
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

#[derive(Debug)]
pub struct NodeList<T: Sized + Node + Clone> {
    ws: WString<T, Stamp>
}
impl<T> NodeList<T> where T: Node + Clone {
    pub fn iter(&self) -> WStringIter<T, Stamp>{
        self.ws.iter()
    }
    pub fn from<I>(io: IoRef, log: Logger, iter: I) -> NodeList<T>
    where I: Iterator<Item=T> {
        trace!(log, "NodeList::from {}", (stringify!(I)));
        let mut ws = WString::new();
        let buf: Vec<u8> = Vec::with_capacity(1000);
        
        for (n, item) in iter.enumerate() {
            trace!(log, "item {}: {:?}", n, item);
            let job = io.create();
            let op = ws.ins(n, item, job.stamp());
            //job.submit(op);
            
            //emit();
        }
        
        NodeList {
            ws: ws
        }
    }
}
impl<T> Node for NodeList<T> where T: Node + Sized + Clone + Into<NodeP> {
    fn childs(&self, out: &mut Vec<NodeP>) {
        for n in self.ws.iter() {
            out.push(n.clone().into());
        }
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        self.iter().map(|n| n.layout(env)).collect()
    }
}
