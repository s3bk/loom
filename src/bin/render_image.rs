extern crate loom;

use std::path::Path;
use std::env;
use loom::output::{Output, png};
use loom::layout::{Writer, GenericWriter};
use loom::io::IoMachine;


fn main() {
    for arg in env::args().skip(1) {
        let path = Path::new(&arg);
        let mut io = IoMachine::new(None);
        let mut output = png::PngOutput::new();
        let mut w: GenericWriter<png::PngOutput> = GenericWriter::new(output.default_font());
        
        io.load_yarn(&path);
        io.layout(&mut w);
        let img = output.render(w.stream(), 600.);
        img.save(path.with_extension("png")).expect("failed to write image");
    }
}

