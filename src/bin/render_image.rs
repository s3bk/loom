extern crate loom;

use std::path::Path;
use std::env;
use loom::render::render;
use loom::layout::TokenStream;
use loom::io::IoMachine;

fn main() {
    for arg in env::args().skip(1) {
        let path = Path::new(&arg);
        let mut io = IoMachine::new(None);
        let mut s = TokenStream::new();
        
        io.load_yarn(&path);
        io.layout(&mut s);
        let img = render(&s, 600.);
        img.save(path.with_extension("png")).expect("failed to write image");
    }
}

