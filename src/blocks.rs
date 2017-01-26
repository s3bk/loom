use std::rc::{Rc, Weak};
use std::cell::RefCell;
use environment::{GraphChain, LocalEnv, Fields, LayoutChain};
use document::*;
use parser;
use io::{Io, AioError};
use layout::{Atom, Glue, Writer};
use commands::{CommandComplete};
use inlinable_string::InlinableString;
use futures::future::{Future, join_all, ok};
use super::LoomError;

type NodeFuture = Box<Future<Item=NodeP, Error=LoomError>>;

fn wrap<N: Node + 'static>(node: N) -> NodeFuture {
    box ok(Ptr::new(node).into())
}

fn process_body(io: Io, env: GraphChain, mut childs: Vec<parser::Body>)
 -> Box<Future<Item=(GraphChain, NodeListP), Error=LoomError>>
{
    use parser::Body;
    
    let nodes = childs.drain(..)
    .map(|node| {
        match node {
            Body::Block(b) => box Pattern::from_block(&io, &env, b),
            Body::Leaf(items) => wrap(Leaf::from(&io, &env, items)),
            Body::List(items) => wrap(List::from(&io, &env, items)),
            Body::Placeholder(p) => wrap(p)
        }
    }).collect::<Vec<_>>();
    
    let io2 = io.clone();
    box join_all(nodes)
    .and_then(move |mut nodes: Vec<NodeP>| {
        let io = io2;
        Ok( (env, Ptr::new(NodeList::from(&io, nodes.drain(..)))) )
    })
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
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
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
    fn layout(&self, _env: LayoutChain, w: &mut Writer) {
        w.punctuation(Atom {
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
    pub fn new(env: &GraphChain, s: &str) -> Symbol {
        let s = match env.get_symbol(s) {
            Some(sym) => sym,
            None => s
        };
        
        Symbol {
            content:    s.into(),
        }
    }
}
impl Node for Symbol {
    fn layout(&self, _env: LayoutChain, w: &mut Writer) {
        w.word(Atom {
            text:   &self.content,
            left:   Glue::None,
            right:  Glue::None
        });
    }
}

fn item_node(io: &Io, env: &GraphChain, i: parser::Item) -> NodeP {
    use parser::Item;
    
    match i {
        Item::Word(ref s) => Ptr::new(Word::new(s)).into(),
        Item::Symbol(ref s) => Ptr::new(Symbol::new(env, s)).into(),
        Item::Punctuation(ref s) => Ptr::new(Punctuation::new(s)).into(),
        Item::Placeholder(p) => Ptr::new(p).into(),
        Item::Token(ref s) => Ptr::new(Word::new(s)).into(),
        Item::Group(g) => Group::from(io, env, g).into()
    }
}

pub struct Group {
    target:     GroupRef,
    fields:     Fields
}

impl Group {
    pub fn from(io: &Io, env: &GraphChain, mut g: parser::Group) -> Ptr<Group> {
        let content = Ptr::new(NodeList::from(io,
            g.content.drain(..).map(|n| item_node(io, env, n))
        ));
        
        let mut g = Ptr::new(Group {
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
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if let Some(target) = self.target.get() {
            target.layout(env.link(self), w)
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
    pub fn from(io: &Io, env: &GraphChain, mut items: Vec<parser::Item>) -> Leaf {
        Leaf {
            content: NodeList::from(io,
                items.drain(..).map(|n| item_node(io, env, n))
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
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if self.content.size() > 0 {
            self.content.layout(env, w);
            w.promote(Glue::Newline { fill: true });
        }
    }
}

struct List {
    items: NodeList<Ptr<Leaf>>
}
impl List {
    pub fn from(io: &Io, env: &GraphChain, mut items: Vec<Vec<parser::Item>>) -> List {
        List {
            items: NodeList::from(
                io,
                items.drain(..).map(|i| Ptr::new(Leaf::from(io, env, i))
            ))
        }
    }
}
impl Node for List {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.items.childs(out)
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        for item in self.items.iter() {
            w.word(Atom {
                left:   Glue::space(),
                right:  Glue::nbspace(),
                text:   "· "
            });
            item.layout(env.clone(), w);
            w.promote(Glue::hfill());
        }
    }
}

fn init_env(io: &Io, env: GraphChain,
    mut commands: Vec<parser::Command>, mut parameters: Vec<parser::Parameter>)
 -> Box<Future<Item=GraphChain, Error=LoomError>>
{
    let io2 = io.clone();
    
    let commands: Vec<_> = commands.drain(..)
        .filter_map(|cmd| {
            match env.get_command(&cmd.name) {
                Some(f) => Some(f(io, &env, cmd.args)),
                None => None
            }
        })
        .collect();
    
    // prepare commands
    let f = join_all(
        commands
    )
    .and_then(move |mut commands: Vec<CommandComplete>| {
        use std::boxed::FnBox;
        
        let mut local_env = LocalEnv::new();
        for c in commands.drain(..) {
            // execute command
            FnBox::call_box(c, (&mut local_env,));
            //c(&mut local_env);
        }
        
        let io = io2;
        
        let definitions = parameters.drain(..)
        .map(|p| Definition::from_param(&io, env.clone(), p))
        .collect::<Vec<_>>();
        
        join_all(definitions)
        .and_then(move |mut items: Vec<Definition>| {
            for d in items.drain(..) {
                local_env.add_target(d.name.clone(), Ptr::new(d).into());
            }
            Ok(env.link(local_env))
        })
    });
    
    box f
}

pub struct Module {
    env:        LocalEnv,
    body:       NodeListP
}
impl Module {
    pub fn parse(io: &Io, env: GraphChain, input: String)
     -> Box< Future<Item=NodeP, Error=LoomError> >
    {
        let (_, body) = parser::block_body(&input, 0).unwrap();
        let io2 = io.clone();
        
        let childs = body.childs;
        box init_env(io, env, body.commands, body.parameters)
        .and_then(move |env| {
            process_body(io2, env, childs)
            .map(|(env, childs)| -> NodeP {
                Ptr::new(Module {
                    env:    env.take(),
                    body:   childs
                }).into()
            })
        })
    }
}
impl Node for Module {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
        self.body.childs(out);
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        self.body.layout(env.link(self), w)
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

pub struct Definition {
    name:       String,

    args:       NodeListP,
    
    // body of the macro declaration
    body:       Ptr<NodeList<NodeP>>,
    
    // referencing macro invocations
    references: RefCell<Vec<Weak<Node>>>,
    
    env:        LocalEnv
}
impl Node for Definition {
    fn childs(&self, out: &mut Vec<NodeP>) {
        out.push(self.args.clone().into());
        out.push(self.body.clone().into());
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        self.args.layout(env.link(self), w);
        self.body.layout(env.link(self), w);
    }
    fn add_ref(&self, source: &Rc<Node>) {
        self.references.borrow_mut().push(Rc::downgrade(source));
    }
    fn env(&self) -> Option<&LocalEnv> {
        Some(&self.env)
    }
}

impl Definition {
    fn from_param(io: &Io, env: GraphChain, p: parser::Parameter)
     -> Box<Future<Item=Definition, Error=LoomError>>
    {
        let io2 = io.clone();
        let args = p.args;
        let name = p.name.to_string();
        let body = p.value;
        let childs = body.childs;
        
        box init_env(io, env, body.commands, body.parameters)
        .and_then(move |env| {
            let io = io2;
            let mut args = args;
            let arglist = Ptr::new(
                NodeList::from(&io,
                    args.drain(..)
                    .map(|n| item_node(&io, &env, n))
                )
            );
            process_body(io, env, childs)
            .and_then(move |(env, childs)| {
                Ok(Definition {
                    name:       name,
                    args:       arglist,
                    body:       childs,
                    references: RefCell::new(vec![]),
                    env:        env.take()
                })
            })
        })
    }
}

pub struct Pattern {
    // the macro itself
    target: Ref,
    
    env: LocalEnv,
    
    fields: Fields
}

impl Pattern {
    fn from_block(io: &Io, env: &GraphChain, block: parser::Block)
     -> Box<Future<Item=NodeP, Error=LoomError>>
    {
        let io2 = io.clone();
        
        let mut argument = block.argument;
        let mut body = block.body;
        let name = block.name.to_string();
        let childs = body.childs;
        
        box init_env(io, env.clone(), body.commands, body.parameters)
        .and_then(move |env| {
            let args = Ptr::new(
                NodeList::from(&io2,
                    argument.drain(..).map(|n| item_node(&io2, &env, n))
                )
            );
            
            process_body(io2, env, childs)
            .map(|(env, body)| -> NodeP {
                let mut p = Ptr::new(Pattern {
                    target:     Ref::new(name).resolve(&env),
                    env:        env.take(),
                    fields:     Fields {
                        args:   Some(args),
                        body:   Some(body)
                    }
                });
                p.into()
            })
        })
    }
}
impl Node for Pattern {
    fn childs(&self, out: &mut Vec<NodeP>) {
        self.env.childs(out);
        self.fields.childs(out);
    }
    fn layout(&self, env: LayoutChain, w: &mut Writer) {
        if let Some(ref target) = self.target.get() {
            target.layout(env.link(self), w);
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
