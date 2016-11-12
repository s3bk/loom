extern crate loom;
extern crate image;

use loom::io::IoMachine;
use std::path::Path;
use loom::layout::{ParagraphLayout, TokenStream};
use loom::render;
use image::{GrayImage, Luma};

fn prepare() -> IoMachine {
    println!("creating IoMachine");
    let mut io = IoMachine::new(None);
    
    io.load_yarn(&Path::new("doc/reference.yarn"));
    
    io
}
fn render(io: IoMachine) {
    let mut s = TokenStream::new();
    io.layout(&mut s);
    
    render::render(&s, 600.9).save("/tmp/test.png").unwrap();
}


#[test]
fn test_render() {
    let io = prepare();
    render(io);
}
