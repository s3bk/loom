use nom::{self, IResult, ErrorKind, digit, Slice, InputLength, InputIter};
use std::iter::{Iterator};
use unicode_categories::UnicodeCategories;
use unicode_brackets::UnicodeBrackets;
use slug;
use super::IString;

macro_rules! alt_apply {
    ($i:expr, $arg:expr, $t:ident $(| $rest:tt)*) =>
    ( alt!($i, apply!($t, $arg) $(| apply!($rest, $arg) )* ) )
}

#[cfg(not(debug_assertions))]
macro_rules! slug {
    ($($t:tt)*) => ()
}

#[cfg(debug_assertions)]
type Data<'a> = slug::Slug<'a>;
        
#[cfg(not(debug_assertions))]
type Data<'a> = &'a str;

#[macro_export]
macro_rules! named (
  ($name:ident, $submac:ident!( $($args:tt)* )) => (
    fn $name<'a>( i: Data<'a> ) -> nom::IResult<Data<'a>, Data<'a>, u32> {
      $submac!(i, $($args)*)
    }
  );
  ($name:ident<$o:ty>, $submac:ident!( $($args:tt)* )) => (
    fn $name<'a>( i: Data<'a> ) -> nom::IResult<Data<'a>, $o, u32> {
      $submac!(i, $($args)*)
    }
  );
);

#[inline(always)]
fn space(input: Data) -> IResult<Data, Data> {
    let mut n = 0;
    for (m, c) in input.iter_elements().enumerate() {
        match c {
            ' ' | '\t' => continue,
            _ => {
                n = m;
                break;
            }
        }
    }
    if n > 0 {
        IResult::Done(input.slice(n ..), input.slice(.. n))
    } else {
        IResult::Error(error_position!(ErrorKind::Space, input))
    }
}
//use nom::space;

#[test]
fn test_space() {
    slug!(
        space("  x") => Done("x", "  ");
        space(" \t \nx") => Done("\nx", " \t ");
        space("\nx") => Error;
        space("x") => Error;
    );
}

#[inline(always)]
fn endline(input: Data) -> IResult<Data, ()> {
    for (m, c) in input.iter_elements().enumerate() {
        match c {
            ' ' | '\t' => continue,
            '\n' => return IResult::Done(input.slice(m + 1 ..), ()),
            _ => break
        }
    }
    IResult::Error(error_position!(ErrorKind::Tag, input))
}
#[test]
fn test_endline() {
    slug!(
        endline("  \n") => Done("", ());
        endline("\t\n") => Done("", ());
        endline("\n") => Done("", ());
    );
}
named!(empty_lines <usize>,
    map!(many1!(endline), |v: Vec<()>| { v.len() } )
);

named!(indent_any,
    alt_complete!(
        tag!("    ")
      | tag!("\t")
    )
);

#[derive(Debug, PartialEq)]
pub enum Placeholder {
    Body,
    Argument(usize),
    Arguments,
    Unknown(IString)
}

named!(placeholder <Placeholder>,
    alt!(
        map!(tag!("body"), {|_| Placeholder::Body })
      | map!(tag!("args"), {|_| Placeholder::Arguments })
      | map_opt!(digit, |s: Data| {
            s.parse::<usize>().ok().map(|n| Placeholder::Argument(n))
        })
    )
);

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

#[inline(always)]
fn is_letter(c: char) -> bool {
    match c {
        'a' ... 'z' => true,
        'A' ... 'Z' => true,
        c if c <= '\u{7E}' => false,
        _ => c.is_letter()
    }
}

#[inline(always)]
fn is_punctuation(c: char) -> bool {
    match c {
        '.' | ',' | ':' | '!' | '?' | ';' => true,
        c if c <= '\u{7E}' => false,
        _ => c.is_punctuation()
    }
}

#[inline(always)]
fn is_symbol(c: char) -> bool {
    match c {
        '+' | '-' | '#' | '*' | '/' | '%' | '&' => true,
        c if c <= '\u{7E}' => false,
        _ => c.is_symbol()
    }
}

#[inline(always)]
fn is_opening(c: char) -> bool {
    match c {
        '(' | '[' | '<' | '{' => true,
        c if c <= '\u{7E}' => false,
        _ => c.is_open_bracket()
    }
}

#[inline(always)]
fn is_closing(c: char) -> bool {
    match c {
        ')' | ']' | '>' | '}' => true,
        c if c <= '\u{7E}' => false,
        _ => c.is_close_bracket()
    }   
}

#[inline(always)]
fn letter_sequence(input: Data) -> IResult<Data, Data> {
    //use unicode_segmentation::UnicodeSegmentation;
    //let gi = UnicodeSegmentation::grapheme_indices(input, true);
    let mut codepoints = input.iter_elements();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(error_position!(ErrorKind::Alpha, input))
    };
    if is_letter(cp) == false {
        return IResult::Error(error_position!(ErrorKind::Alpha, input));
    }
    loop {
        let remaining = codepoints.as_str();
        match codepoints.next() {
            Some(cp) => {
                if is_letter(cp) {
                    continue;
                } else {
                    let p = input.input_len() - remaining.input_len();
                    return IResult::Done(input.slice(p..), input.slice(..p));
                }
            }
            None => break
        }
    }
    
    IResult::Done(input.slice(input.input_len() ..), input)
}

#[test]
fn test_letter_sequence() {
    slug!(
        letter_sequence("hello") => Done("", "hello");
        letter_sequence("h") => Done("", "h");
        letter_sequence("hello world") => Done(" world", "hello");
        letter_sequence("hello\nworld") => Done("\nworld", "hello");
        letter_sequence("") => Error;
    );
}

fn string_esc<'a>(input: Data<'a>) -> IResult<Data<'a>, IString> {
    use std::iter::FromIterator;
    map!(input, many1!(
        complete!(alt!(
            map!(take_until_either!("\\\""), { |d: Data<'a>| d.into() })
          | map!(tag!(r"\\"),     { |_| "\\" })
          | map!(tag!(r"\t"),     { |_| "\t" })
          | map!(tag!(r"\n"),     { |_| "\n" })
          | map!(tag!(r"\ "),     { |_| " "  })
          | map!(tag!(r##"\""##), { |_| "\"" })
        ))),
        |v: Vec<&str>| IString::from_iter(v.iter().flat_map(|s| s.chars()))
    )
}

named!(string <IString>,
    alt!(
        complete!(delimited!(tag!("\""), string_esc, tag!("\"")))
      | map!(take_until_either!("\" \t\n"), |s: Data| s.into())
    )
);
#[test]
fn test_string() {
    slug!(
        string("hallo ") => Done(" ", IString::from("hallo"));
        string("hallo welt") => Done(" welt", IString::from("hallo"));
        string("<hallo >") => Done(" >", IString::from("<hallo"));
        string(r"hallo\ welt") => Done(r" welt", IString::from(r"hallo\"));
        string(r##""hallo welt""##) => Done("", IString::from("hallo welt"));
        string(r##""hallo\ welt" .."##) => Done(" ..", IString::from(r"hallo welt"));
        string(r##""hallo\nwelt""##) => Done("", IString::from("hallo\nwelt"));
    );
}

#[inline(always)]
fn test_chars<'a, F: Fn(char) -> bool>(input: Data<'a>, test: F) -> IResult<Data<'a>, Data<'a>>
{
    let mut codepoints = input.iter_elements();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(error_position!(ErrorKind::Alpha, input))
    };
    if test(cp) == false {
        return IResult::Error(error_position!(ErrorKind::Alpha, input));
    }
    
    loop {
        let p = input.input_len() - codepoints.as_str().len();
        match codepoints.next() {
            Some(cp) => {
                if test(cp) {
                    continue;
                } else {
                    return IResult::Done(input.slice(p..), input.slice(..p));
                }
            }
            None => break
        }
    }
    
    IResult::Done(input.slice(input.input_len() ..), input)
}

named!(item_word <Item>,
    map!(letter_sequence, |s: Data<'a>| { Item::Word(s.into()) })
);
named!(item_symbol <Item>,
    map!(apply!(test_chars, is_symbol ),
        |s: Data<'a>| { Item::Symbol(s.into()) }
    )
);
named!(item_placeholder <Item>,
    map!(preceded!(tag!("$"), placeholder),
        |v| { Item::Placeholder(v) }
    )
);
named!(item_punctuation <Item>,
    map!(apply!(test_chars, is_punctuation ),
        |s: Data<'a>| { Item::Punctuation(s.into()) }
    )
);
named!(item_token <Item>,
    map!(preceded!(tag!("\\"), letter_sequence),
        |s: Data<'a>| { Item::Token(s.into()) }
    )
);
named!(item_group <Item>,
    do_parse!(
        opening:    apply!(test_chars, is_opening)
    >>              opt!(space)
    >>  content:    separated_nonempty_list!(space, item)
    >>              opt!(space)
    >>  closing:    apply!(test_chars, is_closing)
    >>             (Item::Group(Group{
                        opening:    opening.into(),
                        closing:    closing.into(),
                        content:    content
                    }))
    )
);

fn item<'a>(input: Data<'a>) -> IResult<Data<'a>, Item> {
    match input.iter_elements().next() {
        Some(c) => match c {
            'a' ... 'z' |
            'A' ... 'Z' => item_word(input),
            '.' | ',' | ':' | '!' | '?' => item_punctuation(input),
            '$' => item_placeholder(input),
            '\\' => item_token(input),
            '<' | '(' | '[' | '{' => item_group(input),
            _ => alt!(input, item_word | item_symbol | item_punctuation | item_group)
        },
        None => return IResult::Incomplete(nom::Needed::Size(1))
    }
}
#[test]
fn test_item() {
    slug!(
        item("<foo>\n") => Done("\n", Item::Group(Group {
            opening: "<".into(),
            content: vec![Item::Word("foo".into())],
            closing: ">".into()
        }));
        item("<foo> baz") => Done(" baz", Item::Group(Group {
            opening: "<".into(),
            content: vec![Item::Word("foo".into())],
            closing: ">".into()
        }));
        item("<foo bar> baz") => Done(" baz", Item::Group(Group {
            opening: "<".into(),
            content: vec![
                Item::Word("foo".into()),
                Item::Word("bar".into())
            ],
            closing: ">".into()
        }));
        item("<baä>\n") => Done("\n", Item::Group(Group {
            opening: "<".into(),
            content: vec![Item::Word("baä".into())],
            closing: ">".into()
        }));
        item("foo baz") => Done(" baz", Item::Word("foo".into()));
        item("$body\n") => Done("\n", Item::Placeholder(Placeholder::Body));
        item("$3\n") => Done("\n", Item::Placeholder(Placeholder::Argument(3)));
        item("\\foo\n") => Done("\n", Item::Token("foo".into()));
        item("\n") => Error;
    );
}

#[inline(always)]
fn leaf_indent(input: Data, expected_indent: usize) -> IResult<Data, Data> {
    recognize!(input,
        preceded!(
            endline,
            count!(indent_any, expected_indent)
        )
    )
}
#[inline(always)]
fn leaf_seperator(input: Data, expected_indent: usize) -> IResult<Data, Data> {
    alt_complete!(input,
        apply!(leaf_indent, expected_indent) |
        space
    )
}
fn leaf(input: Data, expected_indent: usize) -> IResult<Data, Vec<Item>> {
    delimited!(input,
        complete!(count!(indent_any, expected_indent)),
        separated_nonempty_list!(
            apply!(leaf_seperator, expected_indent),
            item
        ),
        endline
    )
}
#[test]
fn test_leaf() {
    slug!(
        leaf("x\n\ne", 0) => Done("\ne", vec![Item::Word("x".into())]);
        leaf("x \n", 0) => Done("", vec![Item::Word("x".into())]);
        leaf("x $args\n", 0) => Done("", vec![
            Item::Word("x".into()),
            Item::Placeholder(Placeholder::Arguments)
        ]);
        leaf("x \ny\n", 0) => Done("", vec![Item::Word("x".into()), Item::Word("y".into())]);
        leaf("Hello world\nThis is the End .\n", 0) => Done("", vec![
            Item::Word("Hello".into()),
            Item::Word("world".into()),
            Item::Word("This".into()),
            Item::Word("is".into()),
            Item::Word("the".into()),
            Item::Word("End".into()),
            Item::Punctuation(".".into())
        ]);
        leaf("        foo\n        bar \n", 2) => Done("", vec![
            Item::Word("foo".into()),
            Item::Word("bar".into())
        ]);
        leaf("\tx  y\n\tz\nq", 1) => Done("q", vec![
            Item::Word("x".into()),
            Item::Word("y".into()),
            Item::Word("z".into())
        ]);
    );
}

fn list_item(input: Data, expected_indent: usize) -> IResult<Data, Vec<Item>> {
    preceded!(input,
        complete!(tuple!(
            count!(indent_any, expected_indent),
            tag!("  - ")
        )),
        separated_nonempty_list!(
            alt_complete!(
                space |
                apply!(leaf_indent, expected_indent + 1)
            ),
            item
        )
    )
}
#[test]
fn test_list_item() {
    slug!(
        list_item("  - hello world", 0) => Done("", vec![
            Item::Word("hello".into()),
            Item::Word("world".into())
        ]);
        list_item("      - hello", 1) => Done("", vec![Item::Word("hello".into())]);
        list_item("  - hello\n    world\n", 0) => Done("\n", vec![
            Item::Word("hello".into()),
            Item::Word("world".into())
        ]);
    );
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

#[inline(always)]
fn block_leaf(input: Data, indent_level: usize) -> IResult<Data, Body> {
    map!(input,
        apply!(leaf, indent_level),
        |items| { Body::Leaf(items) }
    )
}

#[inline(always)]
fn block_list(input: Data, indent_level: usize) -> IResult<Data, Body> {
    //println!("list_item at level {}", indent_level + 1);
    map!(input,
        many1!(
            apply!(list_item, indent_level)
        ),
        |l| Body::List(l)
    )
}

#[inline(always)]
fn block_block(input: Data, indent_level: usize) -> IResult<Data, Body> {
    map!(input,
        apply!(block, indent_level),
        |b| Body::Block(b)
    )
}

#[inline(always)]
fn block_placeholder(input: Data, indent_level: usize) -> IResult<Data, Body> {
    do_parse!(input,
            count!(indent_any, indent_level)
    >>      tag!("$")
    >> var: placeholder
    >>      endline
    >>     (Body::Placeholder(var))
    )
}
#[test]
fn test_block_placeholder() {
    slug!(
        block_placeholder("    $0\n", 1) =>
            Done("", Body::Placeholder(Placeholder::Argument(0)));
    );
}

#[inline(always)]
fn body(input: Data, indent_level: usize) -> IResult<Data, Body> {
    alt_apply!(input, indent_level,
        block_leaf | block_list | block_block | block_placeholder
    )
}

#[inline(always)]
fn childs(input: Data, indent_level: usize) -> IResult<Data, Vec<Body>> {
    many0!(input, terminated!(
            apply!(body, indent_level),
            opt!(empty_lines)
    ))
}
pub fn block_body(input: Data, indent_level: usize) -> IResult<Data, BlockBody> {
    do_parse!(input,
          commands: many0!(apply!(command, indent_level))
    >>  parameters: many0!(apply!(pattern, indent_level))
    >>      childs: apply!(childs, indent_level)
    >>             (BlockBody {
                        commands:   commands,
                        parameters: parameters,
                        childs:     childs,
                    })
    )
}

#[inline(always)]
pub fn command(input: Data, indent_level: usize) -> IResult<Data, Command> {
    do_parse!(input,
                complete!(count!(indent_any, indent_level))
    >>          tag!("!")
    >>    name: letter_sequence
    >>          opt!(space)
    >>    args: separated_list!(space, string)
    >>          endline
    >>          opt!(empty_lines)
    >>         (Command { name: name.into(), args: args })
    )
}
#[test]
fn test_command() {
    slug!(
        command("!foo \"<bar\" \"baz>\"\n", 0) => Done("", Command {
            name: "foo".into(),
            args: vec![
                "<bar".into(),
                "baz>".into()
            ]
        });
    );
}

#[inline(always)]
pub fn pattern(input: Data, indent_level: usize) -> IResult<Data, Parameter> {
    do_parse!(input,
              complete!(count!(indent_any, indent_level))
    >>        tag!("/")
    >>  name: letter_sequence
    >>        opt!(space)
    >>  args: separated_list!(space, item)
    >>        endline
    >> value: apply!(block_body, indent_level + 1)
    >>       (Parameter { name: name.into(), args: args, value: value })
    )
}

#[test]
fn test_pattern_1() {
    slug!(
        pattern("/foo x\n", 0) => Done("", Parameter {
            name:   "foo".into(),
            args:   vec![Item::Word("x".into())],
            value:  BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![],
            }
        });
    );
}
#[test]
fn test_pattern_2() {
    slug!(
        pattern("/foo x\n    bar\nx", 0) => Done("x", Parameter {
            name:   "foo".into(),
            args:   vec![Item::Word("x".into())],
            value:  BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("bar".into())
                    ])
                ]
            }
        });
    );
}
#[test]
fn test_pattern_3() {
    slug!(
        pattern("/foo x\n    bar $0\nx", 0) => Done("x", Parameter {
            name:   "foo".into(),
            args:   vec![Item::Word("x".into())],
            value:  BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("bar".into()),
                        Item::Placeholder(Placeholder::Argument(0))
                    ])
                ]
            }
        });
    );
}

#[test]
fn test_separated_list() {
    named!(list <Vec<Item> >, separated_list!(space, item));
    slug!(
        list("\n") => Done("\n", Vec::<Item>::new());
    );
}

named!(args <Vec<Item> >,
    alt!(
        map!(endline, |_| { Vec::new() } )
      | delimited!(
            space,
            separated_list!(space, item),
            endline
        )
    )
);

pub fn block(input: Data, indent_level: usize) -> IResult<Data, Block> {
    //println!("block at level {}:", indent_level);
    do_parse!(input,
                complete!(count!(indent_any, indent_level))
    >>          complete!(tag!(":"))
    >>    name: letter_sequence
    >>     arg: args
    >>          opt!(empty_lines)
    >>    body: complete!(apply!(block_body, indent_level + 1))
    >>         (Block {
                    name:       name.into(),
                    argument:   arg,
                    body:       body
                })
    )
}
#[test]
fn test_block_1() {
    slug!(
        block(":foo\n    x\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                    ])
                ]
            }
        });
    );
}
#[test]
fn test_block_2() {
    slug!(
        block(":foo\n\n    x\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                    ])
                ]
            }
        });
    );
}
#[test]
fn test_block_3() {
    slug!(
        block(":foo\n    !x\n    x\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![],
            body: BlockBody {
                commands: vec![
                    Command {
                        name:   "x".into(),
                        args:   vec![]
                    }
                ],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                    ])
                ]
            }
        });
        block(":foo A\n    !x\n    x\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![Item::Word("A".into())],
            body: BlockBody {
                commands: vec![
                    Command {
                        name:   "x".into(),
                        args:   vec![]
                    }
                ],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                    ])
                ]
            }
        });
        block(":foo A\n    :bar\n    x\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![Item::Word("A".into())],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Block(Block {
                        name:       "bar".into(),
                        argument:   vec![],
                        body: BlockBody {
                            commands:   vec![],
                            parameters: vec![],
                            childs:     vec![]
                        }
                    }),
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                    ])
                ]
            }
        });
        
        block(":foo A\n    :bar\n\n    x  y\n\tz\nx", 0) => Done("x", Block {
            name:       "foo".into(),
            argument:   vec![Item::Word("A".into())],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Block(Block {
                        name:       "bar".into(),
                        argument:   vec![],
                        body: BlockBody {
                            commands:   vec![],
                            parameters: vec![],
                            childs:     vec![]
                        }
                    }),
                    Body::Leaf(vec![
                        Item::Word("x".into()),
                        Item::Word("y".into()),
                        Item::Word("z".into()),
                    ])
                ]
            }
        });
    );
}
