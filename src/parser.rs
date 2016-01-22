use nom::{space, newline, IResult, ErrorKind};
use std::str;
use std::iter::{Iterator};

macro_rules! test_parser {
    (
        $fun:ident, $testcase:expr, $remaining:expr, $result:expr
    ) => {
        assert_eq!(
            $fun($testcase),
            IResult::Done($remaining as &[u8], $result)
        )
    }
}

named!(empty_line <&[u8], ()>,
    complete!(chain!(
        many0!(space) ~
        newline,
        || {}
    ))
);
#[test]
fn test_empty_line() {
    test_parser!(empty_line, b"  \n", b"", ());
    test_parser!(empty_line, b"\t\n", b"", ());
    test_parser!(empty_line, b"\n", b"", ());
}
named!(empty_lines <&[u8], usize>,
    map!(many1!(empty_line), |v: Vec<()>| { v.len() } )
);

named!(indent_any,
    complete!(
        alt!(
            tag!(b"    ")
          | tag!(b"\t")
        )
    )
);

#[derive(Debug, PartialEq)]
pub enum Item<'a> {
    Word(&'a str),
    Reference(&'a str)
}

fn printable_byte(b: u8) -> bool {
    match b {
        b' ' | b'\t' | b'\n' => false,
        _ => true
    }
}
named!(printable_sequence <&[u8], &str>,
    map_res!(
        take_while1!(printable_byte),
        str::from_utf8
    )
);
#[test]
fn test_printable_sequence() {
    test_parser!(printable_sequence, b"hello", b"", "hello");
}
#[test]
#[should_panic]
fn test_printable_sequence_2() {
    test_parser!(printable_sequence, b"", b"", "");
}
named!(reference <&[u8], Item>,
    chain!(
        tag!(".")       ~
        name: printable_sequence ~
        space?,
        || { Item::Reference(name) }
    )
);
named!(word <&[u8], Item>,
    chain!(
        w: printable_sequence ~
        space?,
        || { Item::Word(w) }
    )
);
#[test]
fn test_item() {
    test_parser!(reference, b".foo\n", b"\n", Item::Reference("foo"));
    test_parser!(reference, b".foo baz", b"baz", Item::Reference("foo"));
    test_parser!(reference, b".ba\xc3\xa4\n", b"\n", Item::Reference("baä"));
    test_parser!(word, b"foo baz", b"baz", Item::Word("foo"));
}

named!(item <&[u8], Item>,
    alt!( reference | word )
);

fn multiline_fixed_indent(input: &[u8], expected_indent: usize) -> IResult<&[u8], Vec<Item> > {
    let mut out: Vec<Item> = vec![];
    println!("i: {}, input: {:?}", expected_indent, input);
    map!(input, many1!(delimited!(
        complete!(count!(indent_any, expected_indent)),
        many1!(chain!(i: item, || { out.push(i) })),
        newline
    )), |lines| { out })
}
#[test]
fn test_multiline_fixed_indent() {
    named!(_0 <&[u8], Vec<Item> >, apply!(multiline_fixed_indent, 0));
    named!(_2 <&[u8], Vec<Item> >, apply!(multiline_fixed_indent, 2));
    
    test_parser!(_0, b"x\n", b"", vec![Item::Word("x")]);
    test_parser!(_0, b"Hello world\nThis is the End.\n", b"", vec![
        Item::Word("Hello"), Item::Word("world"), Item::Word("This"),
        Item::Word("is"), Item::Word("the"), Item::Word("End.")]);
    test_parser!(_2, b"        foo\n", b"", vec![Item::Word("foo")]);
}

#[derive(Debug, PartialEq)]
pub enum Body<'a> {
    Leaf(Vec<Item<'a>>),
    Block(Block<'a>)
}

#[derive(Debug, PartialEq)]
pub struct Block<'a> {
    pub name: &'a str,
    pub header: Vec<Item<'a>>,
    pub body: Vec<Body<'a>>
}

pub fn block(input: &[u8], indent_level: usize) -> IResult<&[u8], Block, u32> {
    chain!(input,
        complete!(tag!(":"))
      ~ name:   printable_sequence
      ~         space?
      ~ header: apply!(multiline_fixed_indent, indent_level)
      ~ empty_lines?
      ~ body:   separated_list!(dbg!(empty_lines), alt!(
                    map!(apply!(multiline_fixed_indent, indent_level + 1),
                         |v| Body::Leaf(v))
                  | map!(apply!(block, indent_level + 1),
                         |b| Body::Block(b))
                )),
        || { Block{ name: name, header: header, body: body } }
    )
}
named!(block0 <&[u8], Block>, apply!(block, 0));
#[test]
fn test_block() {
    test_parser!(block0, b"\
:hello world
", b"",
        Block{ name: "hello", header: vec![Item::Word("world")], body: vec![] }
    );
}

named!(yarn <&[u8], Vec<Block> >,
       separated_list!(dbg!(empty_lines), block0)
);

#[test]
fn test_yarn() {
    test_parser!(yarn, b"\
:hello world

:foo
    bar
    baz .b\xc3\xa4r
",
        b"", vec![
Block{ name: "hello", header: vec![Item::Word("world")], body: vec![] },
Block{ name: "foo", header: vec![], body: vec![
    Body::Leaf(vec![
        Item::Word("bar"), Item::Word("baz"), Item::Reference("bär")
    ])
]}    
        ]);
}
