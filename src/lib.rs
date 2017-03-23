#![feature(trace_macros)]
#![feature(conservative_impl_trait)]
#![feature(box_syntax)]
#![feature(custom_attribute)]
#![feature(unboxed_closures)]
#![feature(fnbox)]
#![feature(link_args)]

#[macro_use] extern crate nom;
#[macro_use] extern crate wheel;
#[macro_use] extern crate serde_derive;

extern crate itertools;
extern crate serde;
extern crate serde_json;
extern crate unicode_categories;
extern crate unicode_brackets;

extern crate roman;
extern crate fst;
extern crate rmp;
extern crate rmp_serialize;
extern crate rustc_serialize;
//extern crate lz4;
extern crate woot;
//extern crate mio;
extern crate futures;
//extern crate curl;
extern crate inlinable_string;
extern crate ordermap;
extern crate num;

#[cfg(feature = "output_png")]
extern crate image;
#[cfg(feature = "output_png")]
extern crate rusttype;

#[cfg(feature = "output_pdf")]
extern crate pdf;

#[cfg(feature = "output_html")]
extern crate sxd_document;

#[cfg(feature = "platform_default")]
extern crate curl;

pub mod blocks;
pub mod environment;
pub mod document;
pub mod hyphenation;
pub mod layout;
pub mod parser;
pub mod io;
pub mod commands;
pub mod output;
//pub mod slug;
pub mod units;
pub mod config;

use wheel::prelude::*;

#[derive(Debug)]
pub enum LoomError {
    FileRead(<File as AsyncRead>::Error),
    DirectoryGetFile(String, <Directory as AsyncDirectory>::Error),
    MissingArg(&'static str),
    Hyphenator(fst::Error),
    MissingItem(String),
    Parser
}
