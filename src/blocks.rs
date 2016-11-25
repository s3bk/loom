use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use environment::{GraphChain, LocalEnv, Fields};
use document::*;
use parser;
use io::IoRef;
use output::Output;
use layout::{Atom, Glue, Writer, Flex};
use inlinable_string::InlinableString;


struct ErrorBlock(String);
impl Node for ErrorBlock {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        w.word(Atom {
            left:   Glue::space(),
            right:  Glue::nbspace(),
            text:   "Error"
        });
        w.word(Atom {
            left:   Glue::nbspace(),
            right:  Glue::space(),
            text:   &self.0
        });
    }
}


/// process the block and return the resulting layoutgraph
fn process_block(io: IoRef, env: GraphChain, b: &parser::Block) -> P<Node> {
    // look up the name
    println!("process_block name: {}", b.name);
    P::from(Pattern::from_block(io, env, b)).into()
}

type DefinitionListP = P<NodeList<P<Definition>>>;

fn process_body(io: IoRef, env: GraphChain, childs: &[parser::Body]) -> P<NodeList<NodeP>> {
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

pub struct Word {
    content:    InlinableString,
}
impl Word {
    pub fn new(s: &str) -> Word {
        Word {
            content:    s.into(),
        }
    }
}
impl Node for Word {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        env.hyphenate(w, Atom {
            text:   &self.content,
            left:   Glue::space(),
            right:  Glue::space()
        });
    }
}

pub struct Punctuation {
    content:    InlinableString,
}
impl Punctuation {
    pub fn new(s: &str) -> Punctuation {
        Punctuation {
            content:    s.into(),
        }
    }
}
impl Node for Punctuation {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        w.word(Atom {
            text:   &self.content,
            left:   Glue::None,
            right:  Glue::space()
        });
    }
}

pub struct Symbol {
    content:    InlinableString,
}
impl Symbol {
    pub fn new(s: &str) -> Symbol {
        Symbol {
            content:    s.into(),
        }
    }
}
impl Node for Symbol {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        w.word(Atom {
            text:   &self.content,
            left:   Glue::space(),
            right:  Glue::space()
        });
    }
}

fn process_placeholder(env: GraphChain, v: &parser::Var) -> Placeholder {
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
fn item_node(io: IoRef, env: GraphChain, i: &parser::Item) -> NodeP {
    use parser::Item;
    
    match i {
        &Item::Word(ref s) => P::new(Word::new(s)).into(),
        &Item::Symbol(ref s) => P::new(Symbol::new(s)).into(),
        &Item::Punctuation(ref s) => P::new(Punctuation::new(s)).into(),
        &Item::Placeholder(ref v) => P::new(process_placeholder(env, v)).into(),
        &Item::Token(ref s) => P::new(TokenNode::from(env, s)).into(),
        &Item::Group(ref g) => Group::from(io, env, g).into()
    }
}

pub struct TokenNode {
  //  token:  TokenStream
}
impl TokenNode {
    fn from(env: GraphChain, name: &str) -> TokenNode {
    /*    let mut token = TokenStream::new();
        match env.get_token(name) {
            Some(ts) => {
                token.extend(ts);
            },
            None => {}
        }*/
        TokenNode {
           // token: token
        }
    }
}
impl Node for TokenNode {
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        //s.extend(&self.token);
    }
}

pub struct Group {
    target:     GroupRef,
    fields:     Fields
}

impl Group {
    pub fn from(io: IoRef, env: GraphChain, g: &parser::Group) -> P<Group> {
        let content = P::new(NodeList::from(io.clone(),
            g.content.iter().map(|n| item_node(io.clone(), env, n))
        ));
        
        let mut g = P::new(Group {
            target:     GroupRef::new(g.opening, g.closing),
            fields:     Fields {
                args:   Some(content),
                body:   None
            }
        });
        {
            let mut gp: &mut Group = g.get_mut().unwrap();
            gp.target.resolve(env);
        }
        g
    }
}

impl Node for Group {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.fields.childs(out)
    }
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        if let Some(target) = self.target.get() {
            let field_link = env.link_fields(&self.fields);
            target.layout(env.with_fields(Some(&field_link)), w)
        } else {
            let open_close = self.target.key();
            
            w.word(Atom {
                left:   Glue::space(),
                right:  Glue::None,
                text:   &open_close.0
            });
            
            match self.fields.args {
                Some(ref n) => n.layout(env, w),
                None => unreachable!()
            }
            
            w.word(Atom {
                left:   Glue::None,
                right:  Glue::space(),
                text:   &open_close.1
            });
        }
    }
}

pub struct Leaf {
    content: NodeList<NodeP>
}
impl Leaf {
    pub fn from(io: IoRef, env: GraphChain, items: &[parser::Item]) -> Leaf {
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
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        if self.content.size() > 0 {
            self.content.layout(env, w);
            w.promote(Glue::Newline { fill: true });
        }
    }
}

struct List {
    items: NodeList<P<Leaf>>
}
impl List {
    pub fn from(io: IoRef, env: GraphChain, items: &[Vec<parser::Item>]) -> List {
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
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        for item in self.items.iter() {
            w.word(Atom {
                left:   Glue::space(),
                right:  Glue::nbspace(),
                text:   "· "
            });
            item.layout(env, w);
            w.promote(Glue::hfill());
        }
    }
}

fn init_env(io: IoRef, env: GraphChain, body: &parser::BlockBody) -> LocalEnv {
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

pub struct Module {
    env:        LocalEnv,
    body:       NodeListP
}
impl Module {
    pub fn parse(io: IoRef, env: GraphChain, input: &str) -> NodeP {
        use nom::IResult;
        /*
        #[cfg(debug_assertions)]
        use slug;
        
        #[cfg(debug_assertions)]
        let input = slug::wrap(input);
        */
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
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        self.body.layout(env.link(&self.env), w)
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

pub struct Definition {
    // the name of the macro
    name:       String,
    
    args:       NodeListP,
    
    // body of the macro declaration
    body:       P<NodeList<NodeP>>,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>,
    
    env:        LocalEnv
}
impl Node for Definition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.args.clone().into());
        out.push(self.body.clone().into());
    }
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        self.args.layout(env.link(&self.env), w);
        self.body.layout(env.link(&self.env), w);
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

impl Definition {
    fn from_param(io: IoRef, env: GraphChain, p: &parser::Parameter) -> Definition {
        let local_env = init_env(io.clone(), env, &p.value);
        let args = P::new(
            NodeList::from(io.clone(),
                p.args.iter()
                .map(|n| item_node(io.clone(), env.link(&local_env), n))
            )
        );
        Definition {
            name:       p.name.to_string(),
            args:       args,
            body:       process_body(io, env.link(&local_env), &p.value.childs),
            references: RefCell::new(vec![]),
            env:        local_env
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

pub struct Pattern {
    // the macro itself
    target: Ref,
    
    env: LocalEnv,
    
    fields: Fields
}

impl Pattern {
    fn from_block(io: IoRef, env: GraphChain, block: &parser::Block) -> NodeP {
        
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
                args:   Some(args),
                body:   Some(body)
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
        self.fields.childs(out);
    }
    fn layout(&self, env: GraphChain, w: &mut Writer) {
        if let Some(ref target) = self.target.get() {
            let field_link = env.link_fields(&self.fields);
            target.layout(env.link(&self.env).with_fields(Some(&field_link)), w);
        } else {
            for s in &["unresolved" as &str, "macro" as &str, self.target.name()] {
                w.word(Atom {
                    left:   Glue::space(),
                    right:  Glue::space(),
                    text:   s
                });
            }
        }
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}
