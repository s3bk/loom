use unicode_categories::UnicodeCategories;
use nom::{space, newline, IResult, is_alphanumeric, alphanumeric, multispace};
use std::str;
    
macro_rules! test_parser {
    (
        $name:ident, $testcase:expr, $remaining:expr, $result:expr
    ) => {
        assert_eq!(
            $name($testcase),
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

use std::iter::IntoIterator;
named!(indent_level <&[u8], usize>,
    map!(
        many0!(
            alt!(
                tag!(b"    ") |
                tag!(b"\t")
            )
        ),
        |v: Vec<&[u8]>| {
            v.len()
        }
    )
);
#[test]
fn test_indent_level() {
    test_parser!(indent_level, b"test", b"test", 0);
    test_parser!(indent_level, b"  - test", b"  - test", 0);
    test_parser!(indent_level, b"    test", b"test", 1);
    test_parser!(indent_level, b"\ttest", b"test", 1);
}

#[derive(Debug, PartialEq)]
enum Item<'a> {
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

#[derive(Debug, PartialEq)]
struct InputLine<'a> {
    indent_level: usize,
    items: Vec<Item<'a>>
}

#[derive(Debug, PartialEq)]
enum Line<'a> {
    Empty(usize),
    Input(InputLine<'a>)
}
named!(line <&[u8], Line>,
    chain!(
        indent: indent_level ~
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

named!(yarn <&[u8], Vec<Line> >,
    many0!(
        alt!(
            line |
            empty_lines
        )
    )
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
