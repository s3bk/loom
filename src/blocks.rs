use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::{HashMap};
use environment::Environment;
use document::*;
use layout::{TokenStream, Flex, LayoutNode};
use typeset::Font;
use parser;
use std::iter::{once, empty};
use std::borrow::{Borrow, BorrowMut};
use io::IoRef;

pub struct LeafBuilder<'a> {
    env:    &'a Environment<'a>,
    space:  Arc<Flex>,
    font:   Arc<Font>,
    stream: TokenStream
}
impl<'a> LeafBuilder<'a> {
    pub fn new(env: &'a Environment) -> LeafBuilder<'a> {
        let font = env.default_font().unwrap().clone();
        LeafBuilder {
            env:    env,
            space:  font.space().flex(2.0),
            font:   font,
            stream: TokenStream::new()
        }
    }
    pub fn newline(mut self) -> LeafBuilder<'a> {
        self.stream.newline();
        self
    }
    pub fn space(mut self) -> LeafBuilder<'a> {
        self.stream.space(self.space.clone());
        self
    }
    pub fn nbspace(mut self) -> LeafBuilder<'a> {
        self.stream.nbspace(self.space.clone());
        self
    }
    pub fn word(mut self, word: &str) -> LeafBuilder<'a> {
        self.stream.word(self.font.measure(word));
        self
    }
    pub fn word_hyphenated(mut self, word: &str) -> LeafBuilder<'a> {
        self.env.hyphenate(&mut self.stream, word, &self.font);
        self
    }
    pub fn token(mut self, name: &str) -> LeafBuilder<'a> {
        self.env.use_token(&mut self.stream, name);
        self
    }
    pub fn build(self) -> LayoutNode {
        self.stream.into()
    }
}

macro_rules! leaf {
    ($env:ident, $($rest:tt)*) => (
        leaf_complete!(LeafBuilder::new($env), $($rest)*)
    )
}
macro_rules! leaf_complete {
    ($builder:expr, newline) => (
        $builder.newline().build()
    );
    ($builder:expr, newline, $($rest:tt)*) => (
        leaf_complete!($builder.newline(), $($rest)*)
    );
    
    ($builder:expr, ~) => (
        $builder.nbspace().build()
    );
    ($builder:expr, ~, $($rest:tt)*) => (
        leaf_complete!($builder.nbspace(), $($rest)*)
    );
    
    ($builder:expr, _) => (
        $builder.space().build()
    );
    ($builder:expr, _, $($rest:tt)*) => (
        leaf_complete!($builder.space(), $($rest)*)
    );
    
    ($builder:expr, / $name:expr) => (
        $builder.token(stringify!($name)).build()
    );
    ($builder:expr, / $name:expr, $($rest:tt)*) => (
        leaf_complete!($builder.token(stringify!($name)), $($rest)*)
    );
    
    ($builder:expr, $x:expr) => (
        $builder.word($x).build()
    );
    ($builder:expr, $x:expr, $($rest:tt)*) => (
        leaf_complete!($builder.word($x), $($rest)*)
    );
}

#[derive(Debug)]
struct Param {
    name:   String,
    value:  P<Leaf>
}
impl Node for Param {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.value.clone().into())
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        unimplemented!()
    }
}
impl Param {
    fn from(io: IoRef, env: &Environment, p: &parser::Parameter) -> Param {
        Param {
            name:   p.name.to_string(),
            value:  P::from(Leaf::from(io, env, &p.value))
        }
    }
}

#[derive(Debug)]
struct ErrorBlock(String);
impl Node for ErrorBlock {
    fn layout(&self, env: &Environment) -> LayoutNode {
        leaf!(env, "Error:", ~, &self.0, newline).into()
    }
}


/// process the block and return the resulting layoutgraph
fn process_block(env: &Environment, b: &parser::Block) -> P<Node> {
    use std::iter::FromIterator;
    
    // look up the name
    match env.get_handler(b.name) {
        Some(handler) => {
            // prepare new environment
            let mut env_inner = env.extend();
            
            // execute commands
            for cmd in b.commands.iter() {
                match env.get_command(cmd.name) {
                    Some(ref c) => {
                        c(&mut env_inner, &cmd.args);
                    },
                    None => println!("command not found: {:?}", cmd)
                }
            }
            
            // run the handler
            handler(&mut env_inner, b)
        },
        None => {
            P::from(ErrorBlock(format!("unresolved '{}'", b.name))).into()
        }
    }
}

fn process_body(io: IoRef, env: &Environment, body: &str, indent: usize) -> NodeP {
    use parser::Body;
    use nom::IResult;
    
    match parser::block_body(body, indent) {
        IResult::Done(rem, childs) => {
            println!("remaining:\n{}", rem);
            let nl: NodeList<NodeP> = NodeList::from(io.clone(), env,
                childs.iter()
                .map(|node| match node {
                    &Body::Block(ref b) => process_block(env, b),
                    &Body::Leaf(ref items) => P::from(Leaf::from(io.clone(), env, &items)).into(),
                    &Body::List(ref items) => P::from(List::from(io.clone(), env, items)).into(),
                    &Body::Placeholder(ref v) => P::from(process_placeholder(v)).into()
                })
            );
            P::from(nl).into()
        },
        IResult::Error(e) => {
            println!("Error: {:?}", e);
            P::from(ErrorBlock(format!("parser error {:?}", e))).into()
        },
        IResult::Incomplete(_) => {
            P::from(ErrorBlock(format!("incomplete"))).into()
        }
    }
}

#[derive(Debug)]
pub enum Role {
    Word,
    Punctuation
}

#[derive(Debug)]
pub struct Word {
    content:    String,
    role:       Role
}
impl Word {
    pub fn new(s: &str, r: Role) -> Word {
        Word {
            content:    s.to_string(),
            role:       r
        }
    }
}
impl Node for Word {
    fn layout(&self, env: &Environment) -> LayoutNode {
        LeafBuilder::new(env).word_hyphenated(&self.content).build()
    }
    fn space(&self) -> (bool, bool) {
        match self.role {
            Role::Word => (true, true),
            Role::Punctuation => (false, true)
        }
    }
}

fn process_placeholder(v: &parser::Var) -> Placeholder {
    use parser::Var;
    
    match v {
        &Var::Name(ref name) => match name {
            &"body" => Placeholder::Body,
            _ => Placeholder::Unknown(name.to_string())
        },
        &Var::Number(n) => Placeholder::Argument(n)
    }
}
fn item_node(i: &parser::Item) -> NodeP {
    use parser::Item;
    
    match i {
        &Item::Word(ref s) |
        &Item::Reference(ref s) => P::from(Word::new(s, Role::Word)).into(),
        &Item::Symbol(ref s) |
        &Item::Punctuation(ref s) => P::from(Word::new(s, Role::Punctuation)).into(),
        &Item::Macro(ref v) => P::from(process_placeholder(v)).into()
    }
}

#[derive(Debug)]
pub struct Leaf {
    items: P<NodeList<NodeP>>
}
impl Leaf {
    pub fn from(io: IoRef, env: &Environment, items: &[parser::Item]) -> Leaf {
        use parser::Var;
        Leaf {
            items: P::from(NodeList::from(io, env, items.iter().map(item_node)))
        }
    }
    pub fn get(&self, n: usize) -> Option<NodeP> {
        self.items.iter().nth(n).cloned()
    }
}
impl Node for Leaf {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.items.childs(out)
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        let mut nodes = vec![];
        let space = env.default_font().unwrap().space().flex(2.0);
        
        let mut it = self.items.iter().map(|i| (i, i.space())).peekable();
        while let Some(&(next, next_space)) = it.peek() {
            let (prev, prev_space) = it.next().unwrap();
            
            nodes.push(prev.layout(env));
            
            if prev_space.1 && next_space.0 {
                let mut s = TokenStream::new();
                s.space(space.clone());
                nodes.push(s.into());
            }
        }
        
        LayoutNode::Nodes(nodes)
    }
}

#[derive(Debug)]
struct List {
    items: NodeList<P<Cached<Leaf>>>
}
impl List {
    pub fn from(io: IoRef, env: &Environment, items: &[Vec<parser::Item>]) -> List {
        List {
            items: NodeList::from(io.clone(), env, items.iter().map(|i|
                P::from(Cached::new(Leaf::from(io.clone(), env, i)))
            ))
        }
    }
}
impl Node for List {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.items.childs(out)
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        let mut nodes = vec![];
        
        for item in self.items.iter() {
            nodes.push(leaf!(env, "· "));
            nodes.push(item.layout(env));
            nodes.push(leaf!(env, newline));
        }
        
        LayoutNode::Nodes(nodes)
    }
}

#[derive(Debug)]
pub struct RootNode {
    child: NodeP
}
impl RootNode {
    pub fn parse(io: IoRef, env: &Environment, s: &str) -> NodeP {
        P::from(RootNode {
            child: process_body(io, env, s, 0)
        }).into()
    }
}
impl Node for RootNode {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.child.clone())
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        self.child.layout(env)
    }
}

fn parameters(io: IoRef, env: &Environment, params: &[parser::Parameter])
-> NodeList<P<Param>> {
    NodeList::from(io.clone(), env, params.iter().map(|p| Param::from(io.clone(), env, p).into()))
}

#[derive(Debug)]
pub struct MacroDefinition {
    // the name of the macro
    name:   String,
    
    // arguments to the macro declaration
    args:   P<Leaf>,
    
    // parameters for the macro declartion
    params: P<NodeList<P<Param>>>,
    
    // body of the macro declaration
    childs: NodeP,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>
}
impl Node for MacroDefinition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.childs.clone());
        out.push(self.args.clone().into());
        out.push(self.params.clone().into());
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        self.childs.layout(env)
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
}

impl MacroDefinition {
    fn from_block(io: IoRef, env: &Environment, b: &parser::Block) -> NodeP {
        P::from(MacroDefinition {
            name:   b.name.to_string(),
            args:   P::from(Leaf::from(io.clone(), env, &b.argument)),
            params: P::from(parameters(io.clone(), env, &b.parameters)),
            childs: process_body(io.clone(), env, b.body, b.indent),
            references: RefCell::new(vec![])
        }).into()
    }
}

#[derive(Debug)]
pub struct MacroInstance {
    // the macro itself
    target: Ref,
    
    // arguments to that macro
    args:   P<Leaf>,
    
    // parameters for the macro
    params: P<NodeList<P<Param>>>,
    
    // body of the macro invocation
    childs: NodeP,
}
impl Macro for MacroInstance {
    fn placeholder_layout(&self, env: &Environment, p: &Placeholder) -> LayoutNode {
        match p {
            &Placeholder::Body          => self.childs.layout(env),
            &Placeholder::Arguments     => self.args.layout(env),
            &Placeholder::Argument(n)   => match self.args.get(n) {
                Some(arg) => arg.layout(env),
                None => leaf!(env,
                            "Argument", _, &format!("{}", n), _, "is", _,
                            "out", _, "of", _, "bounds", ".", newline)
            },
            &Placeholder::Unknown(ref name)  =>
                leaf!(env, "Name", _, &name, _, "unknown", ".", newline)
        }
    }
}
impl MacroInstance {
    fn from_block(io: IoRef, env: &Environment, b: &parser::Block) -> NodeP {
        let mut p = P::from(MacroInstance {
            args:   P::from(Leaf::from(io.clone(), env, &b.argument)),
            params: P::from(parameters(io.clone(), env, &b.parameters)),
            childs: process_body(io.clone(), env, b.body, b.indent),
            target: Ref::new(b.name.to_string())
        });
        {
            let mut mi: &mut MacroInstance = p.get_mut().unwrap();
            mi.target.resolve(env);
        }
        p.into()
    }
}
impl Node for MacroInstance {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.childs.clone());
        out.push(self.args.clone().into());
        out.push(self.params.clone().into());
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        // the Environment will solve it.
        if let Some(ref target) = self.target.get() {
            let mut env2 = env.extend();
            env2.set_macro(self);
            target.layout(&env2)
        } else {
            leaf!(env, "Unresolved", _, "macro", _, "'", self.target.name(), "'", newline)
        }
    }
}
