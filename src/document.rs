use std::iter::Iterator;
use std::ops::Deref;
use std::cell::RefCell;
use std::rc::Rc;
use layout::{Atom, Writer};
use environment::{LocalEnv, GraphChain, Fields, LayoutChain};
use io::{Stamp, Io, AioError};
use woot::WString;
use inlinable_string::InlinableString;
use futures::{future, Future, BoxFuture};
pub use parser::Placeholder;

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
    fn layout(&self, env: LayoutChain, w: &mut Writer);
    
    /// ?
    fn add_ref(&self, _: &Rc<Node>) {}
    
    fn env(&self) -> Option<&LocalEnv> {
        None
    }
    fn fields(&self) -> Option<&Fields> {
        None
    }
}

pub struct Ptr<N: ?Sized + Node> {
    inner: Rc<N>,
    //references: LinkedList<Ptr<N>>
}
impl<N: ?Sized> Node for Ptr<N> where N: Node {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.inner.childs(out)
    }
    fn modified(&self) {
        self.inner.modified()
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        self.inner.layout(env, w)
    }
}
impl<N> Ptr<N> where N: Node {
    pub fn new(n: N) -> Ptr<N> {
        Ptr {
            inner: Rc::new(n),
            //references: LinkedList::new()
        }
    }
}
impl<N> From<Ptr<N>> for Ptr<Node> where N: Node + Sized + 'static {
    fn from(n: Ptr<N>) -> Ptr<Node> {
        Ptr {
            inner: n.inner as Rc<Node>
        }
    }
}

impl<N: ?Sized> Deref for Ptr<N> where N: Node {
    type Target = N;
    
    fn deref(&self) -> &N {
        self.inner.deref()
    }
}
impl<N> Ptr<N> where N: Node {
    pub fn get_mut(&mut self) -> Option<&mut N> {
        Rc::get_mut(&mut self.inner)
    }
}
impl<N: ?Sized> Clone for Ptr<N> where N: Node {
    fn clone(&self) -> Ptr<N> {
        Ptr {
            inner: self.inner.clone()
        }
    }
}

impl Node for Placeholder {
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        let n = {
            let fields = env.fields()
            .expect("Placeholder::layout: no fields found!");
            let n: Option<NodeP> = match self {
                &Placeholder::Body => fields.body.clone().map(|n| n.into()),
                &Placeholder::Argument(i) => fields.args.clone()
                    .and_then(|n| n.iter().nth(i).cloned()),
                &Placeholder::Arguments => fields.args.clone().map(|n| n.into()),
                _ => None
            };
            n
        };
        n.map(|n| n.layout(env, w))
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
    pub fn resolve(self, env: &GraphChain) -> Ref {
        *self.target.borrow_mut() = env.get_target(&self.name).cloned();
        self
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
    pub fn new(opening: InlinableString, closing: InlinableString) -> GroupRef {
        GroupRef {
            key:    (opening, closing),
            target: RefCell::new(None)
        }
    }
    pub fn resolve(&mut self, env: &GraphChain) {
        if let Some(target) = env.get_group(&self.key) {
            *self.target.borrow_mut() = Some(target.clone());
        }
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
    pub fn from<I>(io: &Io, iter: I) -> NodeList<T>
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
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        for n in self.iter() {
            n.layout(env.clone(), w);
        }
    }
}
