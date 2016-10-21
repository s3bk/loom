use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use environment::Environment;
use document::*;
use layout::{TokenStream, Flex};
use typeset::Font;
use parser;
use io::IoRef;

pub struct LeafBuilder<'a> {
    env:    &'a Environment<'a>,
    space:  Arc<Flex>,
    font:   Arc<Font>,
    stream: &'a mut TokenStream
}
impl<'a> LeafBuilder<'a> {
    pub fn new(env: &'a Environment, s: &'a mut TokenStream) -> LeafBuilder<'a> {
        let font = env.default_font().unwrap().clone();
        LeafBuilder {
            env:    env,
            space:  font.space().flex(2.0),
            font:   font,
            stream: s
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
}

macro_rules! leaf {
    ($env:ident, $stream:ident << $($rest:tt)*) => (
        leaf_complete!(LeafBuilder::new($env, $stream), $($rest)*)
    )
}
macro_rules! leaf_complete {
    ($builder:expr) => (
        {$builder;}
    );
    ($builder:expr, newline $($rest:tt)*) => (
        leaf_complete!($builder.newline() $($rest)*)
    );
    ($builder:expr, ~ $($rest:tt)*) => (
        leaf_complete!($builder.nbspace() $($rest)*)
    );
    ($builder:expr, _ $($rest:tt)*) => (
        leaf_complete!($builder.space() $($rest)*)
    );
    ($builder:expr, / $name:ident $($rest:tt)*) => (
        leaf_complete!($builder.token(stringify!($name)) $($rest)*)
    );
    ($builder:expr, - $x:expr) => (
        {$builder.word_hyphenated($x);}
    );
    ($builder:expr, - $x:expr, $($rest:tt)*) => (
        leaf_complete!($builder.word_hyphenated($x), $($rest)*)
    );
    ($builder:expr, $x:expr) => (
        {$builder.word($x);}
    );
    ($builder:expr, $x:expr, $($rest:tt)*) => (
        leaf_complete!($builder.word($x), $($rest)*)
    );
}

#[derive(Debug)]
struct ErrorBlock(String);
impl Node for ErrorBlock {
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        leaf!(env, s << "Error:", ~, &self.0, newline);
    }
}


/// process the block and return the resulting layoutgraph
fn process_block(io: IoRef, env: &Environment, b: &parser::Block) -> P<Node> {
    // look up the name
    println!("process_block name: {}", b.name);
    P::from(Pattern::from_block(io, env, b)).into()
}

type DefinitionListP = P<NodeList<P<Definition>>>;
fn defines(io: IoRef, env: &mut Environment, params: &[parser::Parameter]) -> DefinitionListP {
    P::new(NodeList::from(io.clone(),
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
    
    P::new(NodeList::from(io.clone(),
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
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        leaf!(env, s << - &self.content);
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
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        let space = env.default_font().unwrap().space().flex(2.0);
        
        let mut it = self.items.iter().map(|i| (i, i.space())).peekable();
        while let Some(&(_, next_space)) = it.peek() {
            let (prev, prev_space) = it.next().unwrap();
            
            prev.layout(env, s);
            
            if prev_space.1 && next_space.0 {
                s.space(space.clone());
            }
        }
        if let Some((last, _)) = it.next() {
            last.layout(env, s);
        }
        s.newline();
    }
}

#[derive(Debug)]
struct List {
    items: NodeList<P<Leaf>>
}
impl List {
    pub fn from(io: IoRef, env: &Environment, items: &[Vec<parser::Item>]) -> List {
        List {
            items: NodeList::from(
                io.clone(),
                items.iter().map(|i| P::new(Leaf::from(io.clone(), env, i))
            ))
        }
    }
}
impl Node for List {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.items.childs(out)
    }
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        for item in self.items.iter() {
            leaf!(env, s << "· ");
            item.layout(env, s);
            s.newline();
        }
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
        use nom::slug::wrap;
        
        #[cfg(release)]
        let input = s;
        
        #[cfg(not(release))]
        let input = wrap(s);
        
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
        
        let mut env = env.extend();
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
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        println!("RootNode::layout()");
        let mut env2 = env.extend();
        for d in self.defines.iter() {
            env2.add_target(d.name(), d.clone().into());
        }
        self.child.layout(&env2, s)
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
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        println!("Definition::layout() {}", self.name);
        self.body.layout(env, s)
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
}

impl Definition {
    fn from_param(io: IoRef, env: &Environment, p: &parser::Parameter) -> Definition {
        Definition {
            name:       p.name.to_string(),
            body:       process_body(io, env, &p.value.childs),
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
    fn placeholder_layout(&self, env: &Environment, s: &mut TokenStream, p: &Placeholder) {
        match p {
            &Placeholder::Body          => self.childs.layout(env, s),
            &Placeholder::Arguments     => self.args.layout(env, s),
            &Placeholder::Argument(n)   => match self.args.get(n) {
                Some(arg) => arg.layout(env, s),
                None => leaf!(env, s <<
                            "Argument", _, &format!("{}", n), _, "is", _,
                            "out", _, "of", _, "bounds", ".", newline)
            },
            &Placeholder::Unknown(ref name)  =>
                leaf!(env, s << "Name", _, &name, _, "unknown", ".", newline)
        }
    }
}
impl Pattern {
    fn from_block(io: IoRef, env: &Environment, block: &parser::Block) -> NodeP {
        use nom::IResult;
        
        let mut inner_env = env.extend();
        
        let ref body = block.body;
        let mut p = P::new(Pattern {
            args:       P::new(Leaf::from(io.clone(), &env, &block.argument)),
            defines:    defines(io.clone(), &mut inner_env, &body.parameters),
            childs:     process_body(io.clone(), &inner_env, &body.childs),
            target:     Ref::new(block.name.to_string())
        });
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
    fn layout(&self, env: &Environment, s: &mut TokenStream) {
        // the Environment will solve it.
        if let Some(ref target) = self.target.get() {
            let mut env2 = env.extend();
            for d in self.defines.iter() {
                env2.add_target(d.name(), d.clone().into());
            }
            env2.set_macro(self);
            target.layout(&env2, s)
        } else {
            leaf!(env, s << "Unresolved", _, "macro", _, "'", self.target.name(),
                "'", /hfill, newline);
        }
    }
}
