use nom::{self, IResult, ErrorKind, digit, AsBytes, Compare, Slice, Offset,
InputLength, IterIndices, Needed, CompareResult, FindToken};
use std::iter::{Iterator};
use unicode_categories::UnicodeCategories;
use unicode_brackets::UnicodeBrackets;

macro_rules! alt_apply {
    ($i:expr, $arg:expr, $t:ident $(| $rest:tt)*) =>
    ( alt!($i, apply!($t, $arg) $(| apply!($rest, $arg) )* ) )
}

#[cfg(not(debug_assertions))]
macro_rules! slug {
    ($($t:tt)*) => ()
}

#[cfg(not(debug_assertions))]
type Data<'a> = &'a str;

#[cfg(debug_assertions)]
type Data<'a> = nom::slug::Slug<'a>;

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

named!(newline, tag!("\n"));

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
pub enum Var<'a> {
    Name(&'a str),
    Number(usize)
}
named!(var <Var>,
    alt!(
        map!(letter_sequence, |s: Data<'a>| { Var::Name(s.into()) }) |
        map_opt!(digit, |s: Data| { s.parse::<usize>().ok().map(Var::Number) })
    )
);

#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub opening: &'a str,
    pub closing: &'a str,
    pub content: Vec<Item<'a>>
}

#[derive(Debug, PartialEq)]
pub enum Item<'a> {
    Word(&'a str),
    Symbol(&'a str),
    Punctuation(&'a str),
    Placeholder(Var<'a>),
    Token(&'a str),
    Group(Group<'a>)
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

fn string_esc<'a>(input: Data<'a>) -> IResult<Data<'a>, String> {
    map!(input, many1!(
        complete!(alt!(
            map!(take_until_either!("\\\""), { |d: Data<'a>| d.into() })
          | map!(tag!(r"\\"),     { |_| "\\" })
          | map!(tag!(r"\t"),     { |_| "\t" })
          | map!(tag!(r"\n"),     { |_| "\n" })
          | map!(tag!(r"\ "),     { |_| " "  })
          | map!(tag!(r##"\""##), { |_| "\"" })
        ))),
        |v: Vec<&str>| v.concat()
    )
}

named!(string <String>,
    alt!(
        complete!(delimited!(tag!("\""), string_esc, tag!("\"")))
      | map!(take_until_either!("\" \t\n"), |s: Data| s.into())
    )
);
#[test]
fn test_string() {
    slug!(
        string("hallo ") => Done(" ", String::from("hallo"));
        string("hallo welt") => Done(" welt", String::from("hallo"));
        string("<hallo >") => Done(" >", String::from("<hallo"));
        string(r"hallo\ welt") => Done(r" welt", String::from(r"hallo\"));
        string(r##""hallo welt""##) => Done("", String::from("hallo welt"));
        string(r##""hallo\ welt" .."##) => Done(" ..", String::from(r"hallo welt"));
        string(r##""hallo\nwelt""##) => Done("", String::from("hallo\nwelt"));
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
    map!(preceded!(tag!("$"), var),
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

fn item<'a>(input: Data<'a>) -> IResult<Data<'a>, Item<'a>> {
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
            opening: "<",
            content: vec![Item::Word("foo")],
            closing: ">"
        }));
        item("<foo> baz") => Done(" baz", Item::Group(Group {
            opening: "<",
            content: vec![Item::Word("foo")],
            closing: ">"
        }));
        item("<foo bar> baz") => Done(" baz", Item::Group(Group {
            opening: "<",
            content: vec![
                Item::Word("foo"),
                Item::Word("bar")
            ],
            closing: ">"
        }));
        item("<baä>\n") => Done("\n", Item::Group(Group {
            opening: "<",
            content: vec![Item::Word("baä")],
            closing: ">"
        }));
        item("foo baz") => Done(" baz", Item::Word("foo"));
        item("$body\n") => Done("\n", Item::Placeholder(Var::Name("body")));
        item("$3\n") => Done("\n", Item::Placeholder(Var::Number(3)));
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
fn leaf<'a>(input: Data<'a>, expected_indent: usize) -> IResult<Data<'a>, Vec<Item<'a>>> {
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
        leaf("x\n\ne", 0) => Done("\ne", vec![Item::Word("x")]);
        leaf("x \n", 0) => Done("", vec![Item::Word("x")]);
        leaf("x $y\n", 0) => Done("", vec![
            Item::Word("x"),
            Item::Placeholder(Var::Name("y"))
        ]);
        leaf("x \ny\n", 0) => Done("", vec![Item::Word("x"), Item::Word("y")]);
        leaf("Hello world\nThis is the End .\n", 0) => Done("", vec![
            Item::Word("Hello"),
            Item::Word("world"),
            Item::Word("This"),
            Item::Word("is"),
            Item::Word("the"),
            Item::Word("End"),
            Item::Punctuation(".")
        ]);
        leaf("        foo\n        bar \n", 2) => Done("", vec![
            Item::Word("foo"),
            Item::Word("bar")
        ]);
        leaf("\tx  y\n\tz\nq", 1) => Done("q", vec![
            Item::Word("x"),
            Item::Word("y"),
            Item::Word("z")
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
            Item::Word("hello"),
            Item::Word("world")
        ]);
        list_item("      - hello", 1) => Done("", vec![Item::Word("hello")]);
        list_item("  - hello\n    world\n", 0) => Done("\n", vec![
            Item::Word("hello"),
            Item::Word("world")
        ]);
    );
}

#[derive(Debug, PartialEq)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub args: Vec<Item<'a>>,
    pub value: BlockBody<'a>
}

#[derive(Debug, PartialEq)]
pub struct Command<'a> {
    pub name: &'a str,
    pub args: Vec<String>
}

#[derive(Debug, PartialEq)]
pub struct Block<'a> {
    pub name:       &'a str,
    pub argument:   Vec<Item<'a>>,
    pub body:       BlockBody<'a>
}

#[derive(Debug, PartialEq)]
pub struct BlockBody<'a> {
    pub commands:   Vec<Command<'a>>,
    pub parameters: Vec<Parameter<'a>>,
    pub childs:     Vec<Body<'a>>
}

#[derive(Debug, PartialEq)]
pub enum Body<'a> {
    Leaf(Vec<Item<'a>>),
    List(Vec<Vec<Item<'a>>>),
    Block(Block<'a>),
    Placeholder(Var<'a>)
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
    >> var: var
    >>      endline
    >>     (Body::Placeholder(var))
    )
}
#[test]
fn test_block_placeholder() {
    slug!(
        block_placeholder("    $foo\n", 1) =>
            Done("", Body::Placeholder(Var::Name("foo")));
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
            name: "foo", args: vec![
                "<bar".to_owned(),
                "baz>".to_owned()
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
            name:   "foo",
            args:   vec![Item::Word("x")],
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
            name:   "foo",
            args:   vec![Item::Word("x")],
            value:  BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("bar")
                    ])
                ]
            }
        });
    );
}
#[test]
fn test_pattern_3() {
    slug!(
        pattern("/foo x\n    bar $baz\nx", 0) => Done("x", Parameter {
            name:   "foo",
            args:   vec![Item::Word("x")],
            value:  BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("bar"),
                        Item::Placeholder(Var::Name("baz"))
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
            name:       "foo",
            argument:   vec![],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x"),
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
            name:       "foo",
            argument:   vec![],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x"),
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
            name:       "foo",
            argument:   vec![],
            body: BlockBody {
                commands: vec![
                    Command {
                        name:   "x",
                        args:   vec![]
                    }
                ],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x"),
                    ])
                ]
            }
        });
        block(":foo A\n    !x\n    x\nx", 0) => Done("x", Block {
            name:       "foo",
            argument:   vec![Item::Word("A")],
            body: BlockBody {
                commands: vec![
                    Command {
                        name:   "x",
                        args:   vec![]
                    }
                ],
                parameters: vec![],
                childs:     vec![
                    Body::Leaf(vec![
                        Item::Word("x"),
                    ])
                ]
            }
        });
        block(":foo A\n    :bar\n    x\nx", 0) => Done("x", Block {
            name:       "foo",
            argument:   vec![Item::Word("A")],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Block(Block {
                        name:       "bar",
                        argument:   vec![],
                        body: BlockBody {
                            commands:   vec![],
                            parameters: vec![],
                            childs:     vec![]
                        }
                    }),
                    Body::Leaf(vec![
                        Item::Word("x"),
                    ])
                ]
            }
        });
        
        block(":foo A\n    :bar\n\n    x  y\n\tz\nx", 0) => Done("x", Block {
            name:       "foo",
            argument:   vec![Item::Word("A")],
            body: BlockBody {
                commands:   vec![],
                parameters: vec![],
                childs:     vec![
                    Body::Block(Block {
                        name:       "bar",
                        argument:   vec![],
                        body: BlockBody {
                            commands:   vec![],
                            parameters: vec![],
                            childs:     vec![]
                        }
                    }),
                    Body::Leaf(vec![
                        Item::Word("x"),
                        Item::Word("y"),
                        Item::Word("z"),
                    ])
                ]
            }
        });
    );
}
