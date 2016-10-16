use nom::{IResult, ErrorKind, digit};
use std::iter::{Iterator};
use unicode_categories::UnicodeCategories;

macro_rules! alt_apply {
    ($i:expr, $arg:expr, $t:ident $(| $rest:tt)*) =>
    ( alt!($i, apply!($t, $arg) $(| apply!($rest, $arg) )* ) )
}

macro_rules! test_parser {
    (
        $fun:ident, $testcase:expr, $remaining:expr, $result:expr
    ) => {
        assert_eq!($fun($testcase as &str), IResult::Done(&$remaining as &str, $result))
    }
}


named!(space <&str, &str>, alt!(tag_s!(" ") | tag_s!("\t")));

named!(newline <&str, &str>, tag_s!("\n"));

named!(endline <&str, ()>,
    complete!(chain!(
        many0!(space) ~
        newline,
        || {}
    ))
);
#[test]
fn test_endline() {
    test_parser!(endline, "  \n", "", ());
    test_parser!(endline, "\t\n", "", ());
    test_parser!(endline, "\n", "", ());
}
named!(empty_lines <&str, usize>,
    map!(many1!(endline), |v: Vec<()>| { v.len() } )
);

named!(indent_any <&str, &str>,
    alt_complete!(
        tag_s!("    ")
      | tag_s!("\t")
    )
);

#[derive(Debug, PartialEq)]
pub enum Var<'a> {
    Name(&'a str),
    Number(usize)
}
named!(var <&str, Var>,
    alt!(
        map!(letter_sequence, |s| { Var::Name(s) }) |
        map_opt!(digit, |s: &str| { s.parse::<usize>().ok().map(Var::Number) })
    )
);


#[derive(Debug, PartialEq)]
pub enum Item<'a> {
    Word(&'a str),
    Reference(&'a str),
    Symbol(&'a str),
    Punctuation(&'a str),
    Macro(Var<'a>)
}

fn letter_sequence(input: &str) -> IResult<&str, &str> {
    //use unicode_segmentation::UnicodeSegmentation;
    //let gi = UnicodeSegmentation::grapheme_indices(input, true);
    let mut codepoints = input.chars();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(error_position!(ErrorKind::Alpha, input))
    };
    if cp.is_letter() == false {
        return IResult::Error(error_position!(ErrorKind::Alpha, input));
    }
    loop {
        let remaining = codepoints.as_str();
        match codepoints.next() {
            Some(cp) => {
                if cp.is_letter() {
                    continue;
                } else {
                    return IResult::Done(remaining, &input[..input.len() - remaining.len()]);
                }
            }
            None => break
        }
    }
    
    IResult::Done("", input)
}
#[test]
fn test_letter_sequence() {
    test_parser!(letter_sequence, "hello", "", "hello");
    test_parser!(letter_sequence, "h", "", "h");
    test_parser!(letter_sequence, "hello world", " world", "hello");
    test_parser!(letter_sequence, "hello\nworld", "\nworld", "hello");
}
#[test]
#[should_panic]
fn test_letter_sequence_2() {
    test_parser!(letter_sequence, "", "", "");
}
named!(reference <&str, Item>,
    chain!(
        tag_s!(".")       ~
        name: letter_sequence,
        || { Item::Reference(name) }
    )
);

fn take_until<'a>(input: &'a str, terminating: &[char]) -> IResult<&'a str, &'a str> {
    let mut codepoints = input.chars();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(error_position!(ErrorKind::Alpha, input))
    };
    if terminating.contains(&cp) {
        return IResult::Error(error_position!(ErrorKind::Alpha, input));
    }
    loop {
        let remaining = codepoints.as_str();
        match codepoints.next() {
            Some(cp) => {
                if terminating.contains(&cp) {
                    return IResult::Done(remaining, &input[..input.len() - remaining.len()]);
                }
            }
            None => break
        }
    }
    
    IResult::Done("", input)
}
#[test]
fn test_take_until() {
    named!(_0 <&str, &str>, apply!(take_until, &['\\', ' ']));
    test_parser!(_0, r"hello\nworld", r"\nworld", "hello");
}

named!(string_esc <&str, String>, 
    map!(many1!(
        complete!(alt!(
            apply!(take_until, &['\\', '"'])
          | map!(tag_s!(r"\\"),     { |_| "\\" })
          | map!(tag_s!(r"\t"),     { |_| "\t" })
          | map!(tag_s!(r"\n"),     { |_| "\n" })
          | map!(tag_s!(r"\ "),     { |_| " "  })
          | map!(tag_s!(r##"\""##), { |_| "\"" })
        ))),
        |v: Vec<&str>| v.concat()
    )
);

named!(string <&str, String>,
    alt!(
        complete!(delimited!(tag_s!("\""), dbg!(string_esc), tag_s!("\"")))
      | map!(apply!(take_until, &['"', ' ', '\t', '\n']), |s: &str| s.to_owned())
    )
);
#[test]
fn test_string() {
    test_parser!(string, "hallo", "", String::from("hallo"));
    test_parser!(string, "hallo welt", " welt", String::from("hallo"));
    test_parser!(string, r"hallo\ welt", r" welt", String::from(r"hallo\"));
    test_parser!(string, r##""hallo welt""##, "", String::from("hallo welt"));
    test_parser!(string, r##""hallo\ welt" .."##, " ..", String::from(r"hallo welt"));
    test_parser!(string, r##""hallo\nwelt""##, "", String::from("hallo\nwelt"));
}


named!(word <&str, Item>,
    map!(letter_sequence, |s| { Item::Word(s) })
);

fn test_chars<'a, F: Fn(char) -> bool>(input: &'a str, test: F) -> IResult<&'a str, &'a str>
{
    let mut codepoints = input.chars();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(error_position!(ErrorKind::Alpha, input))
    };
    if test(cp) == false {
        return IResult::Error(error_position!(ErrorKind::Alpha, input));
    }
    
    loop {
        let remaining = codepoints.as_str();
        match codepoints.next() {
            Some(cp) => {
                if test(cp) {
                    continue;
                } else {
                    return IResult::Done(remaining, &input[..input.len() - remaining.len()]);
                }
            }
            None => break
        }
    }
    
    IResult::Done("", input)
}

named!(symbol <&str, Item>,
    map!(
        apply!(test_chars, |c: char| c.is_symbol() ),
        |s| { Item::Symbol(s) }
    )
);
named!(punctuation <&str, Item>,
    map!(
        apply!(test_chars, |c: char| c.is_punctuation() ),
        |s| { Item::Punctuation(s) }
    )
);
named!(macro_var <&str, Item>,
    map!(
        preceded!(tag_s!("!"), var),
        |v| { Item::Macro(v) }
    )
);

#[test]
fn test_item() {
    test_parser!(reference, ".foo\n", "\n", Item::Reference("foo"));
    test_parser!(reference, ".foo baz", " baz", Item::Reference("foo"));
    test_parser!(reference, ".baä\n", "\n", Item::Reference("baä"));
    test_parser!(word, "foo baz", " baz", Item::Word("foo"));
}

named!(item <&str, Item>,
    alt!( reference | word | symbol | punctuation | macro_var )
);

fn spaces(input: &str) -> IResult<&str, Vec<&str>> {
    many1!(input, space)
}
fn leaf_indent(input: &str, expected_indent: usize) -> IResult<&str, Vec<&str>> {
    preceded!(input,
        endline,
        count!(indent_any, expected_indent)
    )
}
fn leaf_seperator(input: &str, expected_indent: usize) -> IResult<&str, Vec<&str>> {
    alt_complete!(input,
        apply!(leaf_indent, expected_indent) |
        spaces
    )
}
fn leaf<'a>(input: &'a str, expected_indent: usize) -> IResult<&'a str, Vec<Item<'a>>> {
    chain!(input,
            complete!(count!(indent_any, expected_indent)) ~
    items:  separated_nonempty_list!(
                apply!(leaf_seperator, expected_indent),
                item
            ) ~
            endline,
        || { items }
    )
}
#[test]
fn test_leaf() {
    named!(_0 <&str, Vec<Item> >, apply!(leaf, 0));
    named!(_2 <&str, Vec<Item> >, apply!(leaf, 2));
    
    test_parser!(_0, "x\n", "", vec![Item::Word("x")]);
    test_parser!(_0, "x \n", "", vec![Item::Word("x")]);
    test_parser!(_0, "x \ny\n", "", vec![Item::Word("x"), Item::Word("y")]);
    test_parser!(_0, "Hello world\nThis is the End .\n", "", vec![
        Item::Word("Hello"), Item::Word("world"), Item::Word("This"),
        Item::Word("is"), Item::Word("the"), Item::Word("End"), Item::Punctuation(".")]);
    test_parser!(_2, "        foo\n        bar \n", "",
                     vec![Item::Word("foo"),Item::Word("bar")]);
}

fn list_item(input: &str, expected_indent: usize) -> IResult<&str, Vec<Item>> {
    preceded!(input,
        complete!(tuple!(
            count!(indent_any, expected_indent),
            tag_s!("  - ")
        )),
        separated_nonempty_list!(
            alt_complete!(
                spaces |
                apply!(leaf_indent, expected_indent + 1)
            ),
            item
        )
    )
}
#[test]
fn test_list_item() {
    named!(_0 <&str, Vec<Item> >, apply!(list_item, 0));
    named!(_1 <&str, Vec<Item> >, apply!(list_item, 1));
    
    test_parser!(_0, "  - hello world", "",
        vec![Item::Word("hello"), Item::Word("world")]);
    test_parser!(_1, "      - hello", "", vec![Item::Word("hello")]);
    test_parser!(_0, "  - hello\n    world", "",
        vec![Item::Word("hello"), Item::Word("world")]);
}

#[derive(Debug, PartialEq)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub value: Vec<Item<'a>>
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
    pub commands:   Vec<Command<'a>>,
    pub parameters: Vec<Parameter<'a>>,
    pub body:       &'a str,
    pub indent:     usize
}
#[derive(Debug, PartialEq)]
pub enum Body<'a> {
    Leaf(Vec<Item<'a>>),
    List(Vec<Vec<Item<'a>>>),
    Block(Block<'a>),
    Placeholder(Var<'a>)
}

fn block_leaf(input: &str, indent_level: usize) -> IResult<&str, Body> {
    map!(input,
        apply!(leaf, indent_level),
        |items| { Body::Leaf(items) }
    )
}
fn block_list(input: &str, indent_level: usize) -> IResult<&str, Body> {
    //println!("list_item at level {}", indent_level + 1);
    map!(input,
        complete!(many1!(
            apply!(list_item, indent_level)
        )),
        |l| Body::List(l)
    )
}
fn block_block(input: &str, indent_level: usize) -> IResult<&str, Body> {
    map!(input,
        complete!(apply!(block, indent_level)),
        |b| Body::Block(b)
    )
}
fn block_placeholder(input: &str, indent_level: usize) -> IResult<&str, Body> {
    chain!(input,
            apply!(leaf_indent, indent_level + 1) ~
            tag_s!("$") ~
        var:  var ~
            endline,
        || { Body::Placeholder(var) }
    )
}

pub fn block_body(input: &str, indent_level: usize) -> IResult<&str, Vec<Body>> {
    many0!(input,
        terminated!(
            alt_apply!(indent_level,
                block_leaf | block_list | block_block | block_placeholder
            ),
            opt!(empty_lines)
        )
    )
}
pub fn command(input: &str, indent_level: usize) -> IResult<&str, Command> {
    complete!(input,
        chain!(
            complete!(count!(indent_any, indent_level+1)) ~
            tag_s!("/:") ~
      name: letter_sequence ~
            spaces ~
      args: separated_list!(spaces, string) ~
            endline
    , || { Command { name: name, args: args } }
        )
    )
}
pub fn parameter(input: &str, indent_level: usize) -> IResult<&str, Parameter> {
    complete!(input,
        chain!(
            complete!(count!(indent_any, indent_level+1)) ~
            tag_s!("/") ~
      name: letter_sequence ~
            spaces ~
     value: separated_list!(spaces, item) ~
            endline
    , || { Parameter { name: name, value: value } }
        )
    )
}

pub fn block(input: &str, indent_level: usize) -> IResult<&str, Block> {
    //println!("block at level {}:", indent_level);
    chain!(input,
                complete!(count!(indent_any, indent_level)) ~
                complete!(tag_s!(":")) ~
          name: letter_sequence ~
                spaces? ~
      argument: separated_list!(spaces, item) ~
                endline ~
      commands: many0!(apply!(command, indent_level)) ~
    parameters: many0!(apply!(parameter, indent_level)) ~
                empty_lines? ~
          body: recognize!(many0!(
                    preceded!(
                        complete!(count!(indent_any, indent_level+1)),
                        take_until_and_consume!("\n")
                    )
                )),
        || {
            Block {
                name:       name,
                argument:   argument,
                commands:   commands,
                parameters: parameters,
                body:       body,
                indent:     indent_level
            }
        }
    )
}
#[test]
fn test_block_1() {
    named!(_0 <&str, Block>, apply!(block, 0));
    
    test_parser!(_0, "\
:hello world
    X
", "",
        Block {
            indent:     0,
            name:       "hello",
            argument:   vec![Item::Word("world")],
            commands:   vec![],
            parameters: vec![],
            body:       "    X\n"
        }
    );
    
    test_parser!(_0, "\
:foo
    /:answer \"40\\ +\\ 2\"
    /x y
",
        "", 
        Block {
            indent:     0,
            name:       "foo",
            argument:   vec![],
            commands:   vec![
                Command { name: "answer", args: vec!["40 + 2".to_owned()] }
            ],
            parameters: vec![
                Parameter { name: "x", value: vec![Item::Word("y")] }
            ],
            body: ""
        }
    );
}


