#![feature(trace_macros)]
#![feature(conservative_impl_trait)]
#![feature(box_syntax)]
#![feature(custom_attribute)]
#![feature(unboxed_closures)]
#![feature(fnbox)]
#![feature(link_args)]

//#[macro_use] extern crate derivative;
#[macro_use] extern crate nom;
#[macro_use] extern crate itertools;
#[macro_use] extern crate yaio;
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

#[derive(Debug)]
pub enum LoomError {
    Io(yaio::AioError),
    MissingArg(&'static str),
    Fst(fst::Error),
    MissingItem(String),
    Parser
}
impl From<yaio::AioError> for LoomError {
    fn from(e: yaio::AioError) -> LoomError {
        LoomError::Io(e)
    }
}
impl From<fst::Error> for LoomError {
    fn from(e: fst::Error) -> LoomError {
        LoomError::Fst(e)
    }
}
