#![feature(test)]
extern crate test;

use test::Bencher;

#[derive(Copy, Clone)]
pub enum AtomType {
    Punctuation,
    Ordinal,
    Open,
    Close,
    Binary,
    Relation,
    Accent,
    AccentWide,
    BotAccent,
    BotAccentWide,
    Alpha,
    Fence,
    Operator(bool),     // bool := limits or nolimits?
    Over,
    Under,
}

// defines SYMBOLS
include!("../../ReX/src/symbols/math_list.rs");


#[bench]
fn lookup_symbol_hashmap(b: &mut Bencher) {
    use std::collections::HashMap;
    use std::iter::FromIterator;
    
    let map: HashMap<String, (char, AtomType)> =
        HashMap::from_iter(SYMBOLS.iter().map(|&(n, c, t)| (n.to_owned(), (c as char, t))));
    
    let names = ["acute", "approx", "cdot", "downtriangleleftblack", "mbffrakb"];
    b.iter(move || {
        for n in names.iter().cloned() {
            let sym = map.get(n).unwrap();
            test::black_box(sym);
        }
    });
}
