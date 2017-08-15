#![feature(test)]

extern crate loom;
extern crate test;
extern crate nom;

use loom::parser;
use test::Bencher;
use nom::IResult;

#[cfg(not(debug_assertions))]
#[inline(always)]
fn wrap(s: &str) -> &str {s}

#[cfg(debug_assertions)]
use nom::slug::wrap;

#[bench]
fn bench_block(b: &mut Bencher) {
    let reference = include_str!("../doc/reference.yarn");
    b.iter(move || match parser::block_body(wrap(reference), 0) {
           IResult::Done("", block) => block,
           e => panic!("{:?}", e)
    });
}
