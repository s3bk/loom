extern crate loom;

use std::path::Path;
use std::env;
use loom::output::*;
use loom::layout::GenericWriter;
use loom::io::IoMachine;

#[cfg(feature = "output_png")]
fn make_png(io: &IoMachine, path: &Path) {
    let mut output = PngOutput::new();
    let mut w: GenericWriter<PngOutput> = GenericWriter::new(output.default_font());
    
    io.layout(&mut w);
    
    let img = output.render(w.finish(), 600.);
    img.save(path.with_extension("png")).expect("failed to write image");
}

#[cfg(feature = "output_pdf")]
fn make_pdf(io: &IoMachine, path: &Path) {
    let mut output = PdfOutput::new(&path.with_extension("pdf"));
    let mut w: GenericWriter<PngOutput> = GenericWriter::new(output.default_font());
    
    io.layout(&mut w);
    
    output.render(w.finish(), 600.);
}

#[cfg(feature = "output_html")]
fn make_html(io: &IoMachine, path: &Path) {
    let mut output = HtmlOutput::new(&path.with_extension("html"));
    let mut w: GenericWriter<HtmlOutput> = GenericWriter::new(output.default_font());
    
    io.layout(&mut w);
    
    output.render(w.finish());
}

fn main() {
    for arg in env::args().skip(1) {
        let path = Path::new(&arg);
        let mut io = IoMachine::new(None);
        io.load_yarn(&path);
        
        #[cfg(feature = "output_png")]
        make_png(&io, &path);
        
        #[cfg(feature = "output_pdf")]
        make_pdf(&io, &path);
        
        #[cfg(feature = "output_html")]
        make_html(&io, &path);
    }
}
