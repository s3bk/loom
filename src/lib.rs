#![feature(trace_macros)]
#![feature(proc_macro)]
#![feature(conservative_impl_trait)]
#![feature(box_syntax)]
#![feature(custom_attribute)]

//#[macro_use] extern crate derivative;
#[macro_use] extern crate nom;
#[macro_use] extern crate itertools;
extern crate unicode_categories;
extern crate unicode_brackets;

extern crate roman;
extern crate fst;
extern crate rmp;
extern crate rmp_serialize;
extern crate rustc_serialize;
extern crate lz4;
extern crate woot;
extern crate mio;
extern crate futures;
extern crate curl;
extern crate inlinable_string;
extern crate ordermap;

#[cfg(feature = "output_png")]
extern crate image;
#[cfg(feature = "output_png")]
extern crate rusttype;

#[cfg(feature = "output_pdf")]
extern crate pdf;

pub mod blocks;
pub mod environment;
pub mod document;
pub mod hyphenation;
pub mod layout;
pub mod parser;
pub mod io;
pub mod commands;
pub mod output;
pub mod slug;
