use istring::IString;
use super::LoomError;
use io::Io;

#[derive(Debug, PartialEq)]
pub enum Placeholder {
    Body,
    Argument(usize),
    Arguments,
    Unknown(IString)
}

#[derive(Debug, PartialEq)]
pub struct Group {
    pub opening: IString,
    pub closing: IString,
    pub content: Vec<Item>
}

#[derive(Debug, PartialEq)]
pub enum Item {
    Word(IString),
    Symbol(IString),
    Punctuation(IString),
    Placeholder(Placeholder),
    Token(IString),
    Group(Group)
}
 

#[derive(Debug, PartialEq)]
pub struct Parameter {
    pub name: IString,
    pub args: Vec<Item>,
    pub value: BlockBody
}

#[derive(Debug, PartialEq)]
pub struct Command {
    pub name: IString,
    pub args: Vec<IString>
}

#[derive(Debug, PartialEq)]
pub struct Block {
    pub name:       IString,
    pub argument:   Vec<Item>,
    pub body:       BlockBody
}

#[derive(Debug, PartialEq)]
pub struct BlockBody {
    pub commands:   Vec<Command>,
    pub parameters: Vec<Parameter>,
    pub childs:     Vec<Body>
}

#[derive(Debug, PartialEq)]
pub enum Body {
    Leaf(Vec<Item>),
    List(Vec<Vec<Item>>),
    Block(Block),
    Placeholder(Placeholder)
}

pub fn parse(io: &Io, input: &str) -> Result<BlockBody, LoomError> {
    use parser;
    use slug;
    
    #[cfg(feature="slug")]
    let input = slug::wrap(input);
        
    match parser::block_body(input, 0) {
        Ok((rem, out)) => {
            if rem.len() > 0 {
                let s: &str = rem.into();
                warn!(io.log, "remaining:\n{}", s);
            }
            Ok(out)
        },
        Err(_) => Err(LoomError::Parser)
    }
}
