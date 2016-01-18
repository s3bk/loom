use nom::{space, newline, IResult, Consumer, ConsumerState, Move, Input};
use std::str;
    
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

named!(new_line <&[u8], ()>,
    chain!(
        many0!(space) ~
        newline,
        || {}
    )
);
#[test]
fn test_new_line() {
    test_parser!(new_line, b"  \n", b"", ());
    test_parser!(new_line, b"\t\n", b"", ());
    test_parser!(new_line, b"\n", b"", ());
}

named!(empty_lines <&[u8], Line>,
    map!(
        many1!(new_line),
        |r: Vec<()>| Line::Empty(r.len())
    )
);
#[test]
fn test_empty_lines() {
    test_parser!(empty_lines, b"  \n\n", b"", Line::Empty(2));
    test_parser!(empty_lines, b"    \n", b"", Line::Empty(1));
}
named!(indent_by_space, tag!(b"    "));
named!(indent_by_tab, tag!(b"\t"));
named!(indent_any, alt!(indent_by_space | indent_by_tab));
named!(p_indent <&[u8], usize>,
    map!(
        many0!(indent_any),
        |v: Vec<&[u8]>| {
            v.len()
        }
    )
);
#[test]
fn test_indent_level() {
    test_parser!(p_indent, b"test", b"test", 0);
    test_parser!(p_indent, b"  - test", b"  - test", 0);
    test_parser!(p_indent, b"    test", b"test", 1);
    test_parser!(p_indent, b"\ttest", b"test", 1);
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item<'a> {
    Word(&'a str),
    Reference(&'a str),
    Command(&'a str)
}
named!(command <&[u8], Item>,
    chain!(
        tag!(":")       ~
        name: map_res!(take_until_either_bytes!(b" \t\n"), str::from_utf8) ~
        space?,
        || { Item::Command(name) }
    )
);
named!(reference <&[u8], Item>,
    chain!(
        tag!(".")       ~
        name: map_res!(take_until_either_bytes!(b" \t\n"), str::from_utf8) ~
        space?,
        || { Item::Reference(name) }
    )
);
named!(word <&[u8], Item>,
    chain!(
        w: map_res!(take_until_either_bytes!(b" \t\n"), str::from_utf8) ~
        space?,
        || { Item::Word(w) }
    )
);
#[test]
fn test_item() {
    test_parser!(command, b":cmd\n", b"\n", Item::Command("cmd"));
    test_parser!(reference, b".foo\n", b"\n", Item::Reference("foo"));
    test_parser!(reference, b".foo baz", b"baz", Item::Reference("foo"));
    test_parser!(reference, b".ba\xc3\xa4\n", b"\n", Item::Reference("baä"));
    test_parser!(word, b"foo baz", b"baz", Item::Word("foo"));
}

named!(item <&[u8], Item>,
    alt!( reference | command | word )
);

#[derive(Debug, PartialEq, Clone)]
pub struct InputLine<'a> {
    indent_level: usize,
    items: Vec<Item<'a>>
}

#[derive(Debug, PartialEq, Clone)]
pub enum Line<'a> {
    Empty(usize),
    Input(InputLine<'a>)
}
named!(line <&[u8], Line>,
    chain!(
        indent: p_indent ~
        items: many1!(item) ~
        newline,
        || {Line::Input(InputLine{ indent_level: indent, items: items })}
    )
);
#[test]
fn test_line() {
    test_parser!(line, b"    foo\n", b"",
        Line::Input(InputLine{indent_level: 1, items: vec![
            Item::Word("foo")
        ]
    }));
    test_parser!(line, b"hello world\n", b"",
        Line::Input(InputLine{indent_level: 0, items: vec![
            Item::Word("hello"),
            Item::Word("world")
        ]
    }));
    test_parser!(line, b".hello world\nbar", b"bar",
         Line::Input(InputLine{indent_level: 0, items: vec![
             Item::Reference("hello"),
             Item::Word("world")
         ]
    }));
}

named!(yarn_line <&[u8], Line>,
    alt!(
        line |
        empty_lines
    )
);

named!(yarn <&[u8], Vec<Line> >,
    many0!(yarn_line)
);
fn multiline_fixed_indent(input: &[u8], expected_indent: usize) -> IResult<&[u8], Vec<Item> > {
    let mut out: Vec<Item> = vec![];
    map!(input, many1!(delimited!(
        count!(indent_any, expected_indent),
        many1!(chain!(i: item, || { out.push(i) })),
        newline
    )), |lines| { out })
}


#[test]
fn test_indent_count() {
    named!(_foo <&[u8], Vec<Item> >, apply!(multiline_fixed_indent, 2));
    test_parser!(_foo, b"        foo\n", b"", vec![Item::Word("foo")]);
}

enum Body<'a> {
    Empty(usize),
    Leaf(Vec<Item<'a>>),
    Block(Block<'a>)
}

struct Block<'a> {
    name: &'a str,
    header: Vec<Item<'a>>,
    body: Vec<Body<'a>>
}

fn block(input: &[u8], indent_level: usize) -> IResult<&[u8], Block, u32> {
    chain!(input,
        tag!(":")
      ~ name:   map_res!(take_until_either_bytes!(b" \t\n"), str::from_utf8)
      ~         space?
      ~ header: apply!(multiline_fixed_indent, indent_level)
      ~ body:   many0!(alt!(
                    map!(many1!(new_line),
                         |r: Vec<()>| Body::Empty(r.len()))
                  | map!(apply!(multiline_fixed_indent, indent_level + 1),
                         |v| Body::Leaf(v))
                  | map!(apply!(block, indent_level + 1),
                         |b| Body::Block(b))
                )),
        || { Block{ name: name, header: header, body: body } }
    )
}


#[test]
fn test_yarn() {
    test_parser!(yarn, b"\
:hello world

:foo
    bar
    baz .b\xc3\xa4r
",
        b"", vec![
        Line::Input(InputLine{indent_level: 0, items: vec![
            Item::Command("hello"),
            Item::Word("world")
        ]}),
        Line::Empty(1),
        Line::Input(InputLine{indent_level: 0, items: vec![
            Item::Command("foo")
        ]}),
        Line::Input(InputLine{indent_level: 1, items: vec![
            Item::Word("bar")
        ]}),
        Line::Input(InputLine{indent_level: 1, items: vec![
            Item::Word("baz"),
            Item::Reference("bär")
        ]}),
    ]);
}

#[derive(Debug)]
pub struct LineConsumer<'a> {
    state: ConsumerState<Line<'a>, (), Move>
}

impl<'a> LineConsumer<'a> {
    pub fn new() -> LineConsumer<'a> {
        LineConsumer{ state: ConsumerState::Continue(Move::Consume(0)) }
    }
}
impl<'a> Consumer<&'a [u8], Line<'a>, (), Move> for LineConsumer<'a> {
    fn handle(&mut self, input: Input<&'a [u8]>)
        -> &ConsumerState<Line<'a>,(),Move> {
        match input {
            Input::Empty | Input::Eof(None) => &self.state,
            Input::Element(sl)              => {
                match yarn_line(sl) {
                    IResult::Error(_) => {
                        self.state = ConsumerState::Error(())
                    },
                    IResult::Incomplete(_) => {
                        self.state = ConsumerState::Continue(Move::Consume(0))
                    },
                    IResult::Done(i,o) => {
                        self.state = 
                            ConsumerState::Done(Move::Consume(i.len()),o)
                    }
                };
                &self.state
            }
            Input::Eof(Some(sl))            => {
                match yarn_line(sl) {
                    IResult::Error(_) => {
                        self.state = ConsumerState::Error(())
                    },
                    IResult::Incomplete(_) => {
                        // we cannot return incomplete on Eof
                        self.state = ConsumerState::Error(())
                    },
                    IResult::Done(i,o) => {
                        self.state = 
                            ConsumerState::Done(Move::Consume(i.len()), o)
                    }
                };
                &self.state
            }
        }

    }

    fn state(&self) -> &ConsumerState<Line<'a>, (), Move> {
        &self.state
    }
}

#[test]
fn test_parser() {
    use nom::{FileProducer, Producer};
    
    let mut p = FileProducer::new("doc/reference.yarn", 1024).unwrap();
    let mut c = LineConsumer::new();
    
    println!("{:?}", p.apply(&mut c));
    // println!("{:?}", p.apply(&mut c));
}
