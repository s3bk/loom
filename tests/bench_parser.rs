#![feature(test)]

extern crate loom;
extern crate test;
extern crate nom;

use loom::parser;
use test::Bencher;
use nom::IResult;

fn recursive_block(s: &str, level: usize) {
    use loom::parser::Body;
    
    let block = parser::block(s, level).unwrap().1;
    for body in parser::block_body(block.body, block.indent).unwrap().1.iter() {
        match body {
            &Body::Block(ref b) => recursive_block(b.body, b.indent),
            &Body::List(ref l) => {test::black_box(l);}
            &Body::Leaf(ref l) => {test::black_box(l);}
            &Body::Placeholder(ref p) => {test::black_box(p);}
        }
    }
    
    test::black_box(block);
}

#[bench]
fn bench_block(b: &mut Bencher) {
    let reference = include_str!("../doc/reference.yarn");
    b.iter(|| recursive_block(test::black_box(reference), 0));
}
