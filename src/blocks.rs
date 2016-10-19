use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use environment::Environment;
use document::*;
use layout::{TokenStream, Flex, LayoutNode};
use typeset::Font;
use parser;
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
struct ErrorBlock(String);
impl Node for ErrorBlock {
    fn layout(&self, env: &Environment) -> LayoutNode {
        leaf!(env, "Error:", ~, &self.0, newline).into()
    }
}


/// process the block and return the resulting layoutgraph
fn process_block(io: IoRef, env: &Environment, b: &parser::Block) -> P<Node> {
    // look up the name
    debug!(env, "process_block name: {}", (b.name));
    P::from(Pattern::from_block(io, env, b)).into()
}

type DefinitionListP = P<NodeList<P<Definition>>>;
fn defines(io: IoRef, env: &mut Environment, params: &[parser::Parameter]) -> DefinitionListP {
    let log = env.logger(o!("NodeList" => "defines"));
    P::new(NodeList::from(io.clone(), log,
        params.iter()
        .map(|p| {
            let d = P::new(Definition::from_param(io.clone(), env, p));
            env.add_target(d.name(), d.clone().into());
            d
        })
    ))
}

fn process_body(io: IoRef, env: &Environment, childs: &[parser::Body]) -> P<NodeList<NodeP>> {
    use parser::Body;
    
    let log = env.logger(o!("NodeList" => "process_body"));
    P::new(NodeList::from(io.clone(), log,
        childs.iter()
        .map(|node| match node {
            &Body::Block(ref b) => process_block(io.clone(), env, b),
            &Body::Leaf(ref items) => P::new(Leaf::from(io.clone(), env, &items)).into(),
            &Body::List(ref items) => P::new(List::from(io.clone(), env, items)).into(),
            &Body::Placeholder(ref v) => P::new(process_placeholder(v)).into()
        })
    ))
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
            &"args" => Placeholder::Arguments,
            _ => Placeholder::Unknown(name.to_string())
        },
        &Var::Number(n) => Placeholder::Argument(n)
    }
}
fn item_node(i: &parser::Item) -> NodeP {
    use parser::Item;
    
    match i {
        &Item::Word(ref s) |
        &Item::Reference(ref s) => P::new(Word::new(s, Role::Word)).into(),
        &Item::Symbol(ref s) |
        &Item::Punctuation(ref s) => P::new(Word::new(s, Role::Punctuation)).into(),
        &Item::Placeholder(ref v) => P::new(process_placeholder(v)).into()
    }
}

#[derive(Debug)]
pub struct Leaf {
    items: P<NodeList<NodeP>>
}
impl Leaf {
    pub fn from(io: IoRef, env: &Environment, items: &[parser::Item]) -> Leaf {
        Leaf {
            items: P::new(NodeList::from(
                io,
                env.logger(o!("NodeList" => "Leaf::from")),
                items.iter().map(item_node)
            ))
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
        while let Some(&(_, next_space)) = it.peek() {
            let (prev, prev_space) = it.next().unwrap();
            
            nodes.push(prev.layout(env));
            
            if prev_space.1 && next_space.0 {
                let mut s = TokenStream::new();
                s.space(space.clone());
                nodes.push(s.into());
            }
        }
        if let Some((last, _)) = it.next() {
            nodes.push(last.layout(env));
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
            items: NodeList::from(
                io.clone(),
                env.logger(o!("NodeList" => "List::from")),
                items.iter().map(|i| P::new(Cached::new(Leaf::from(io.clone(), env, i)))
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
    defines:    DefinitionListP,
    child:      NodeP
}
impl RootNode {
    pub fn parse(io: IoRef, env: &Environment, s: &str) -> NodeP {
        use nom::IResult;
        
        let input = parser::wrap(s, env.logger(vec![]));
        let b = match parser::block_body(input, 0) {
            IResult::Done(rem, b) => {
                println!("{:?}", rem);
                b
            },
            IResult::Error(e) => {
                println!("{:?}", e);
                panic!();
            },
            _ => panic!()
        };
        
        let mut env = env.extend(o!());
        P::new(RootNode {
            defines:    defines(io.clone(), &mut env, &b.parameters),
            child:      process_body(io, &env, &b.childs).into()
        }).into()
    }
}
impl Node for RootNode {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.defines.clone().into());
        out.push(self.child.clone());
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        trace!(env, "RootNode::layout()");
        let mut env2 = env.extend(o!("env" => "RootNode::layout"));
        for d in self.defines.iter() {
            env2.add_target(d.name(), d.clone().into());
        }
        self.child.layout(&env2)
    }
}

#[derive(Debug)]
pub struct Definition {
    // the name of the macro
    name:   String,
    
    // body of the macro declaration
    body: P<NodeList<NodeP>>,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>
}
impl Node for Definition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.body.clone().into());
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        trace!(env, "Definition::layout() {}", (self.name));
        self.body.layout(env)
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
}

impl Definition {
    fn from_param(io: IoRef, env: &Environment, p: &parser::Parameter) -> Definition {
        let mut env = env.extend(o!("define" => (p.name.to_string())));
        Definition {
            name:       p.name.to_string(),
            body:       process_body(io, &env, &p.value.childs),
            references: RefCell::new(vec![])
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub struct Pattern {
    // the macro itself
    target: Ref,
    
    // arguments to that macro
    args:   P<Leaf>,
    
    // parameters for the macro
    defines: DefinitionListP,
    
    // body of the macro invocation
    childs: P<NodeList<NodeP>>,
}
impl Macro for Pattern {
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
impl Pattern {
    fn from_block(io: IoRef, env: &Environment, block: &parser::Block) -> NodeP {
        use nom::IResult;
        
        let mut inner_env = env.extend(o!("pattern" => (block.name.to_string())));
        
        let ref body = block.body;
        let mut p = P::new(Pattern {
            args:       P::new(Leaf::from(io.clone(), &env, &block.argument)),
            defines:    defines(io.clone(), &mut inner_env, &body.parameters),
            childs:     process_body(io.clone(), &inner_env, &body.childs),
            target:     Ref::new(block.name.to_string())
        });
        trace!(inner_env, "resolving");
        {
            let mut mi: &mut Pattern = p.get_mut().unwrap();
            mi.target.resolve(env);
        }
        p.into()
    }
}
impl Node for Pattern {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.childs.clone().into());
        out.push(self.args.clone().into());
        out.push(self.defines.clone().into());
    }
    fn layout(&self, env: &Environment) -> LayoutNode {
        // the Environment will solve it.
        if let Some(ref target) = self.target.get() {
            let mut env2 = env.extend(o!("env" => "Definition"));
            for d in self.defines.iter() {
                env2.add_target(d.name(), d.clone().into());
            }
            env2.set_macro(self);
            target.layout(&env2)
        } else {
            leaf!(env, "Unresolved", _, "macro", _, "'", self.target.name(),
                "'", /hfill, newline)
        }
    }
}
