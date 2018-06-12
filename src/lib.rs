#![feature(trace_macros)]
#![feature(box_syntax)]
#![feature(unboxed_closures)]
#![feature(fnbox)]
#![feature(link_args)]
#![feature(proc_macro)]
#![feature(generators)]

#[macro_use] extern crate nom;
#[macro_use] extern crate wheel;
#[macro_use] extern crate serde_derive;

extern crate itertools;
extern crate serde;
extern crate serde_json;
extern crate unicode_categories;
extern crate unicode_brackets;
extern crate marksman_escape;

extern crate roman;
extern crate fst;
extern crate rmp;
extern crate rmp_serialize;
extern crate woot;
extern crate futures_await as futures;
extern crate istring;
extern crate indexmap;
extern crate num;
extern crate tuple;

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

#[macro_use]
pub mod slug;

pub mod nodes;
pub mod environment;
pub mod document;
pub mod hyphenation;
pub mod layout;
pub mod parser;
pub mod io;
pub mod commands;
pub mod output;
pub mod units;
pub mod config;
pub mod book;
pub mod source;

use wheel::prelude::*;
use istring::IString;

#[derive(Debug)]
pub enum LoomError {
    FileRead(<File as AsyncRead>::Error),
    DirectoryGetFile(<Directory as AsyncDirectory>::Error),
    DirectoryOpen(<Directory as AsyncOpen>::Error),
    ConfigError(config::ParseError),
    MissingArg(&'static str),
    Hyphenator(fst::Error),
    MissingItem(IString),
    Parser
}

#[allow(unused)]
pub mod prelude {
    pub use output::*;
    pub use io::*;
    pub use config::*;
    pub use LoomError;
    pub use layout::*;
    pub use futures::prelude::*;
}
