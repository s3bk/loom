#![feature(test)]

extern crate loom;
extern crate test;
extern crate nom;

use loom::parser;
use test::Bencher;

#[bench]
fn bench_block(b: &mut Bencher) {
    let reference = include_str!("../doc/reference.yarn");
    b.iter(move || parser::block_body(nom::slug::wrap(reference), 0));
}
