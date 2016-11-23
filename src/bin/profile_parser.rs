#![feature(test)]

extern crate loom;
extern crate test;
extern crate nom;

use loom::parser;
use test::black_box;

//#[cfg(not(debug_assertions))]
#[inline(always)]
fn wrap(s: &str) -> &str {s}

/*
#[cfg(debug_assertions)]
use loom::slug::wrap;
*/
fn main() {
    let reference = include_str!("../../doc/reference.yarn");
    for i in 0 .. 10 {
        println!("{}", i);
        for i in 0 .. 1_000_000 {
            black_box(parser::block_body(wrap(reference), 0));
        }
    }
}
