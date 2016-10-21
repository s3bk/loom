extern crate loom;
extern crate image;

use loom::io::IoMachine;
use std::path::Path;
use loom::layout::{ParagraphLayout, TokenStream};
use image::{GrayImage, Luma};

fn prepare() -> IoMachine {
    println!("creating IoMachine");
    let mut io = IoMachine::new(None);
    
    io.load_yarn(&Path::new("doc/reference.yarn"));
    
    io
}
fn render(io: IoMachine) {
    let width = 600.0;
    println!("creating layout");
    
    let mut s = TokenStream::new();
    io.layout(&mut s);
    println!("flattening layout");
    let mut p = ParagraphLayout::new(s, width);
    
    println!("calculating layout");
    let lines = p.run();

    let margin_v = 10.0;
    let margin_h = 10.0;
    
    let height: f32 = lines.iter().map(|l| l.height).sum();
    let mut image = GrayImage::from_pixel(
        (width + 2. * margin_h) as u32,
        (height + 2. * margin_v) as u32,
        Luma { data: [255u8] }
    );
    
    println!("rendering");
    
    let mut y = margin_v;
    for line in lines.iter() {
        y += line.height;
        for &(ref word, x) in line.words.iter() {
            word.draw_at(&mut image, (x+margin_h, y));
        }
    }
    
    println!("saving");
    image.save("/tmp/test.png");
}


#[test]
fn test_render() {
    let io = prepare();
    render(io);
}
