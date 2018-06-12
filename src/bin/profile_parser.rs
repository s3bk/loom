#![feature(test)]

extern crate loom;
extern crate test;
extern crate nom;

use loom::parser;
use test::black_box;

fn main() {
    let input = include_str!("../../doc/reference.yarn");
    
    #[cfg(feature="slug")]
    let input = slug::wrap(input);
    
    for i in 0 .. 10 {
        println!("{}", i);
        for _ in 0 .. 1_000_000 {
            black_box(parser::block_body(input, 0));
        }
    }
}
