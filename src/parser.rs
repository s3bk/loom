use nom::{IResult, ErrorKind, Err, eof};
use std::iter::{Iterator};
use unicode_categories::UnicodeCategories;

macro_rules! test_parser {
    (
        $fun:ident, $testcase:expr, $remaining:expr, $result:expr
    ) => {
        assert_eq!(
            $fun($testcase as &str),
            IResult::Done($remaining as &str, $result)
        )
    }
}

named!(space <&str, &str>, alt!(tag_s!(" ") | tag_s!("\t")));
named!(newline <&str, &str>, tag_s!("\n"));

named!(empty_line <&str, ()>,
    complete!(chain!(
        many0!(space) ~
        newline,
        || {}
    ))
);
#[test]
fn test_empty_line() {
    test_parser!(empty_line, "  \n", "", ());
    test_parser!(empty_line, "\t\n", "", ());
    test_parser!(empty_line, "\n", "", ());
}
named!(empty_lines <&str, usize>,
    map!(many1!(empty_line), |v: Vec<()>| { v.len() } )
);

named!(indent_any <&str, &str>,
    alt_complete!(
        tag_s!("    ")
      | tag_s!("\t")
    )
);

#[derive(Debug, PartialEq)]
pub enum Item<'a> {
    Word(&'a str),
    Reference(&'a str),
    Symbol(&'a str),
    Punctuation(&'a str)
}

fn letter_sequence(input: &str) -> IResult<&str, &str> {
    //use unicode_segmentation::UnicodeSegmentation;
    //let gi = UnicodeSegmentation::grapheme_indices(input, true);
    let mut codepoints = input.chars();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(Err::Position(ErrorKind::Alpha, input))
    };
    if cp.is_letter() == false {
        return IResult::Error(Err::Position(ErrorKind::Alpha, input));
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
named!(word <&str, Item>,
    map!(letter_sequence, |s| { Item::Word(s) })
);

fn test_chars<'a, F: Fn(char) -> bool>(input: &'a str, test: F) -> IResult<&'a str, &'a str>
{
    let mut codepoints = input.chars();
    let cp = match codepoints.next() {
        Some(cp) => cp,
        None => return IResult::Error(Err::Position(ErrorKind::Alpha, input))
    };
    if test(cp) == false {
        return IResult::Error(Err::Position(ErrorKind::Alpha, input));
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

#[test]
fn test_item() {
    test_parser!(reference, ".foo\n", "\n", Item::Reference("foo"));
    test_parser!(reference, ".foo baz", " baz", Item::Reference("foo"));
    test_parser!(reference, ".baä\n", "\n", Item::Reference("baä"));
    test_parser!(word, "foo baz", " baz", Item::Word("foo"));
}

named!(item <&str, Item>,
    terminated!(
        alt!( reference | word | symbol | punctuation ),
        alt_complete!( peek!( space ) | peek!( newline ) | eof )
    )
);

fn leaf_space(input: &str) -> IResult<&str, Vec<&str>> {
    many1!(input, space)
}
fn leaf_indent(input: &str, expected_indent: usize)-> IResult<&str, Vec<&str>> {
    preceded!(input, tag_s!("\n"), count!(indent_any, expected_indent))
}
fn leaf<'a>(input: &'a str, expected_indent: usize) -> IResult<&'a str, Vec<Item<'a>>> {
    preceded!(input,
        complete!(count!(indent_any, expected_indent)),
        separated_nonempty_list!(
            alt_complete!(
                leaf_space |
                apply!(leaf_indent, expected_indent)
            ),
            item
        )
    )
}
#[test]
fn test_leaf() {
    named!(_0 <&str, Vec<Item> >, apply!(leaf, 0));
    named!(_2 <&str, Vec<Item> >, apply!(leaf, 2));
    
    test_parser!(_0, "x", "", vec![Item::Word("x")]);
    test_parser!(_0, "Hello world\nThis is the End .\n", "", vec![
        Item::Word("Hello"), Item::Word("world"), Item::Word("This"),
        Item::Word("is"), Item::Word("the"), Item::Word("End"), Item::Punctuation(".")]);
    test_parser!(_2, "        foo\n", "\n", vec![Item::Word("foo")]);
}

fn list_item(input: &str, expected_indent: usize) -> IResult<&str, Vec<Item>> {
    preceded!(input,
        complete!(tuple!(
            count!(indent_any, expected_indent),
            tag_s!("  - ")
        )),
        separated_nonempty_list!(
            alt_complete!(
                leaf_space |
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
pub struct Block<'a> {
    pub name: &'a str,
    pub header: Vec<Item<'a>>,
    pub body: Vec<Body<'a>>
}
#[derive(Debug, PartialEq)]
pub enum Body<'a> {
    Leaf(Vec<Item<'a>>),
    List(Vec<Vec<Item<'a>>>),
    Block(Block<'a>)
}

fn block_leaf(input: &str, indent_level: usize) -> IResult<&str, Body> {
    map!(input,
        apply!(leaf, indent_level + 1),
        |items| { Body::Leaf(items) }
    )
}
fn block_list(input: &str, indent_level: usize) -> IResult<&str, Body> {
    println!("list_item at level {}", indent_level + 1);
    map!(input,
        complete!(many1!(dbg!(
            apply!(list_item, indent_level + 1)
        ))),
        |l| Body::List(l)
    )
}
fn block_block(input: &str, indent_level: usize) -> IResult<&str, Body> {
    map!(input,
        complete!(apply!(block, indent_level + 1)),
        |b| Body::Block(b)
    )
}
pub fn block(input: &str, indent_level: usize) -> IResult<&str, Block> {
    println!("block at level {}:\n{}", indent_level, input);
    chain!(input,
        complete!(count!(indent_any, indent_level))
      ~ complete!(tag_s!(":"))
      ~ name:   letter_sequence
      ~         space?
      ~ header: separated_list!(leaf_space, item)
      ~ empty_lines?
      ~ body:   many0!(chain!(
                  b: alt!(
                        dbg!(apply!(block_leaf, indent_level))
                      | dbg!(apply!(block_list, indent_level))
                      | dbg!(apply!(block_block, indent_level))
                    )
                  ~ empty_lines?,
                  || { b }
                )),
        || { Block{ name: name, header: header, body: body } }
    )
}
#[test]
fn test_block_1() {
    named!(_0 <&str, Block>, apply!(block, 0));
    
    test_parser!(_0, "\
:hello world
", "",
        Block{ name: "hello", header: vec![Item::Word("world")], body: vec![] }
    );
    
    test_parser!(_0, "\
:foo
    bar
    baz .bär
    
      - listitem
",
        "", 
        Block{ name: "foo", header: vec![], body: vec![
            Body::Leaf(vec![
                Item::Word("bar"), Item::Word("baz"), Item::Reference("bär"),
            ]),
            Body::List(vec![
                vec![Item::Word("listitem")]
            ])
        ]}
    );
}
