use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use environment::{Environment, LocalEnv, Fields};
use document::*;
use layout::{TokenStream, Flex};
use typeset::Font;
use parser;
use io::IoRef;

pub struct LeafBuilder<'a> {
    env:    Environment<'a>,
    space:  Arc<Flex>,
    font:   Arc<Font>,
    stream: &'a mut TokenStream
}
impl<'a> LeafBuilder<'a> {
    pub fn new(env: Environment<'a>, s: &'a mut TokenStream) -> LeafBuilder<'a> {
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
        if let Some(ts) = self.env.get_token(name) {
            self.stream.extend(ts);
        }
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
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        leaf!(env, s << "Error:", ~, &self.0, newline);
    }
}


/// process the block and return the resulting layoutgraph
fn process_block(io: IoRef, env: Environment, b: &parser::Block) -> P<Node> {
    // look up the name
    println!("process_block name: {}", b.name);
    P::from(Pattern::from_block(io, env, b)).into()
}

type DefinitionListP = P<NodeList<P<Definition>>>;

fn process_body(io: IoRef, env: Environment, childs: &[parser::Body]) -> P<NodeList<NodeP>> {
    use parser::Body;
    
    P::new(NodeList::from(io.clone(),
        childs.iter()
        .map(|node| match node {
            &Body::Block(ref b) => process_block(io.clone(), env, b),
            &Body::Leaf(ref items) => P::new(Leaf::from(io.clone(), env, &items)).into(),
            &Body::List(ref items) => P::new(List::from(io.clone(), env, items)).into(),
            &Body::Placeholder(ref v) => P::new(process_placeholder(env, v)).into()
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
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        let (left, right) = self.space();
        let font = env.default_font().unwrap();
        if s.maybe_space(left, right) {
            s.space(font.space().flex(2.0));
        }
        env.hyphenate(s, &self.content, &font);
    }
    fn space(&self) -> (bool, bool) {
        match self.role {
            Role::Word => (true, true),
            Role::Punctuation => (false, true)
        }
    }
}

fn process_placeholder(env: Environment, v: &parser::Var) -> Placeholder {
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
fn item_node(io: IoRef, env: Environment, i: &parser::Item) -> NodeP {
    use parser::Item;
    
    match i {
        &Item::Word(ref s) => P::new(Word::new(s, Role::Word)).into(),
        &Item::Symbol(ref s) |
        &Item::Punctuation(ref s) => P::new(Word::new(s, Role::Punctuation)).into(),
        &Item::Placeholder(ref v) => P::new(process_placeholder(env, v)).into(),
        &Item::Token(ref s) => P::new(TokenNode::from(env, s)).into(),
        &Item::Group(ref g) => P::new(Group::from(io, env, g)).into()
    }
}

#[derive(Debug)]
pub struct TokenNode {
    token:  TokenStream
}
impl TokenNode {
    fn from(env: Environment, name: &str) -> TokenNode {
        let mut token = TokenStream::new();
        match env.get_token(name) {
            Some(ts) => {
                token.extend(ts);
            },
            None => {
                let ref mut s = token;
                leaf!(env, s << &format!("\\{}", name));
            }
        }
        TokenNode {
            token: token
        }
    }
}
impl Node for TokenNode {
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        s.extend(&self.token);
    }
}

#[derive(Debug)]
pub struct Group {
    content: NodeList<NodeP>,
    opening: String,
    closing: String,
}
impl Group {
    pub fn from(io: IoRef, env: Environment, g: &parser::Group) -> Group {
        Group {
            content:    NodeList::from(io.clone(),
                g.content.iter().map(|n| item_node(io.clone(), env, n))
            ),
            opening:    g.opening.to_owned(),
            closing:    g.closing.to_owned()
        }
    }
}

impl Node for Group {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.content.childs(out)
    }
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        let font = env.default_font().expect("no default font set");
        let space = font.space();
        
        if s.maybe_space(true, false) {
            s.space(space.clone());
        }
        s.word(font.measure(&self.opening));
        
        self.content.layout(env, s);
        
        if s.maybe_space(false, true) {
            s.space(space.clone());
        }
        s.word(font.measure(&self.closing));
    }
}

#[derive(Debug)]
pub struct Leaf {
    content: NodeList<NodeP>
}
impl Leaf {
    pub fn from(io: IoRef, env: Environment, items: &[parser::Item]) -> Leaf {
        Leaf {
            content: NodeList::from(io.clone(),
                items.iter().map(|n| item_node(io.clone(), env, n))
            )
        }
    }
    pub fn get(&self, n: usize) -> Option<NodeP> {
        self.content.iter().nth(n).cloned()
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a NodeP> {
        self.content.iter()
    }
}
impl Node for Leaf {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.content.childs(out)
    }
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        self.content.layout(env, s);
        leaf!(env, s << /hfill, newline);
    }
}

#[derive(Debug)]
struct List {
    items: NodeList<P<Leaf>>
}
impl List {
    pub fn from(io: IoRef, env: Environment, items: &[Vec<parser::Item>]) -> List {
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
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        for item in self.items.iter() {
            leaf!(env, s << "· ");
            item.layout(env, s);
        }
    }
}

fn init_env(io: IoRef, env: Environment, body: &parser::BlockBody) -> LocalEnv {
    let mut local_env = LocalEnv::new();
    for cmd in body.commands.iter() {
        println!("command: {}", cmd.name);
        match env.get_command(cmd.name) {
            Some(c) => {
                c(io.clone(), env, &mut local_env, &cmd.args);
            },
            None => println!("command '{}' not found", cmd.name)
        }
    }
    for p in body.parameters.iter() {
        let d = P::new(Definition::from_param(io.clone(), env.link(&local_env), p));
        local_env.add_target(p.name, d.into());
    }
    local_env
}

#[derive(Debug)]
pub struct Module {
    env:        LocalEnv,
    body:       NodeListP
}
impl Module {
    pub fn parse(io: IoRef, env: Environment, s: &str) -> NodeP {
        use nom::IResult;
        use nom::slug::wrap;
        
        #[cfg(not(debug_assertions))]
        let input = s;
        
        #[cfg(debug_assertions)]
        let input = wrap(s);
        
        let body = match parser::block_body(input, 0) {
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
        
        let mut local_env = init_env(io.clone(), env, &body);
        let body = process_body(io, env.link(&local_env), &body.childs);
        
        P::new(Module {
            env:    local_env,
            body:   body
        }).into()
    }
}
impl Node for Module {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
        self.body.childs(out);
    }
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        println!("Module::layout()");
        self.body.layout(env.link(&self.env), s)
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

#[derive(Debug)]
pub struct Definition {
    // the name of the macro
    name:       String,
    
    // body of the macro declaration
    body:       P<NodeList<NodeP>>,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>,
    
    env:        LocalEnv
}
impl Node for Definition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.body.clone().into());
    }
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        println!("Definition::layout() {}", self.name);
        self.body.layout(env.link(&self.env), s)
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
}

impl Definition {
    fn from_param(io: IoRef, env: Environment, p: &parser::Parameter) -> Definition {
        let local_env = init_env(io.clone(), env, &p.value);
        Definition {
            name:       p.name.to_string(),
            body:       process_body(io, env.link(&local_env), &p.value.childs),
            references: RefCell::new(vec![]),
            env:        local_env
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
    
    env: LocalEnv,
    
    fields: Fields
}

impl Pattern {
    fn from_block(io: IoRef, env: Environment, block: &parser::Block) -> NodeP {
        
        let mut local_env = init_env(io.clone(), env, &block.body);
        let args = P::new(
            NodeList::from(io.clone(),
                block.argument.iter().map(|n| item_node(io.clone(), env.link(&local_env), n))
            )
        );
        
        let body = process_body(io, env.link(&local_env), &block.body.childs);
        
        let mut p = P::new(Pattern {
            target:     Ref::new(block.name.to_string()),
            env:        local_env,
            fields:     Fields {
                args:   args,
                body:   body
            }
        });
        
        { // don't ask
            let mut mi: &mut Pattern = p.get_mut().unwrap();
            mi.target.resolve(env);
        }
        p.into()
    }
}
impl Node for Pattern {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
    }
    fn layout(&self, env: Environment, s: &mut TokenStream) {
        if let Some(ref target) = self.target.get() {
            let field_link = env.link_fields(&self.fields);
            target.layout(env.link(&self.env).with_fields(Some(&field_link)), s)
        } else {
            leaf!(env, s << "Unresolved", _, "macro", _, "'", self.target.name(),
                "'", /hfill, newline);
        }
    }
}
