mod aside;
mod block;
mod definition;
mod group;
mod leaf;
mod list;
mod module;
mod token;

mod prelude {
    pub use document::*;
    pub use std::cell::RefCell;
    pub use std::rc::{Rc, Weak};
    pub use environment::*;
    pub use layout::*;
    pub use io::Io;
    pub use source;
    pub use LoomError;
    pub use futures::future::{Future, join_all, ok};
    pub use nodes::*;
    pub use istring::IString;
}
pub use self::aside::*;
pub use self::block::*;
pub use self::definition::*;
pub use self::group::*;
pub use self::leaf::*;
pub use self::list::*;
pub use self::module::*;
pub use self::token::*;
use self::prelude::*;

use std::fmt;
use commands::{CommandComplete};
use wheel::Log;

type NodeFuture = Box<Future<Item=NodeP, Error=LoomError>>;

fn wrap<N: Node + 'static>(node: N) -> NodeFuture {
    box ok(Ptr::new(node).into())
}

fn process_body(io: Io, env: GraphChain, childs: Vec<source::Body>)
 -> Box< Future<Item=(GraphChain, NodeListP), Error=LoomError> >
{
    use source::Body;
    
    let io2 = io.clone();
    let nodes = childs.into_iter()
    .map(|node| {
        match node {
            Body::Block(b) => box Block::from_block(&io, &env, b),
            Body::Leaf(items) => wrap(Leaf::from(&io, &env, items)),
            Body::List(items) => wrap(List::from(&io, &env, items)),
            Body::Placeholder(p) => wrap(p)
        }
    }).collect::<Vec<_>>();
    
    box join_all(nodes)
    .and_then(move |nodes: Vec<NodeP>| {
        let io = io2;
        Ok( (env, Ptr::new(NodeList::from(&io, nodes.into_iter()))) )
    })
}

pub struct Word {
    content:    IString,
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
impl fmt::Debug for Word {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"w"{}""#, self.content)
    }
}

pub struct Punctuation {
    content:    IString,
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
impl fmt::Debug for Punctuation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"p"{}""#, self.content)
    }
}

pub struct Symbol {
    content:    IString,
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
impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"s"{}""#, self.content)
    }
}

fn item_node(io: &Io, env: &GraphChain, i: source::Item) -> NodeP {
    use source::Item;
    
    match i {
        Item::Word(ref s) => Ptr::new(Word::new(s)).into(),
        Item::Symbol(ref s) => Ptr::new(Symbol::new(env, s)).into(),
        Item::Punctuation(ref s) => Ptr::new(Punctuation::new(s)).into(),
        Item::Placeholder(p) => Ptr::new(p).into(),
        Item::Token(s) => Ptr::new(Token::new(s)).into(),
        Item::Group(g) => Group::from(io, env, g).into()
    }
}

fn init_env(io: Io, env: GraphChain,
    commands: Vec<source::Command>, parameters: Vec<source::Parameter>)
 -> Box<Future<Item=GraphChain, Error=LoomError>>
{
    let log = io.log;
    
    let commands: Vec<_> = commands.into_iter()
        .filter_map(|cmd| {
            match env.get_command(&cmd.name) {
                Some(f) => match f(&io, &env, cmd.args) {
                    Ok(f) => Some(f),
                    Err(_) => None,
                },
                None => {
                    trace!(log, "not found");
                    None
                }
            }
        })
        .collect();
    
    // prepare commands
    let f = join_all(
        commands
    )
    .and_then(move |commands: Vec<CommandComplete>| {
        use std::boxed::FnBox;
        
        let mut local_env = LocalEnv::new();
        for c in commands.into_iter() {
            // execute command
            FnBox::call_box(c, (&env, &mut local_env,));
            //c(&mut local_env);
        }
        
        let definitions = parameters.into_iter()
        .map(|p| Definition::from_param(io.clone(), env.clone(), p))
        .collect::<Vec<_>>();
        
        join_all(definitions)
        .and_then(move |items: Vec<Definition>| {
            for d in items.into_iter() {
                local_env.add_target(d.name().into(), Ptr::new(d).into());
            }
            Ok(env.link(local_env))
        })
    });
    
    box f
}

