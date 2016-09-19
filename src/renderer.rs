use rusttype as rt;
use layout;
use document;
use environment::{Environment, prepare_environment};
use std::sync::Arc;
use std::fs::File;
use std::io::Read;
use layout::{TokenStream, Line};
use image::{GrayImage, Luma};


#[test]
fn test_format() {
    let mut doc = document::Document::new();
    let root = doc.parse("doc/reference.yarn");
    
    let mut env = Environment::new();
    prepare_environment(&mut env);
    
    let mut s = TokenStream::new();
    env.process_block(&root, &mut s);
    let width = 600.0;
    let margin = 50.0;
    for (n, lines) in layout::ParagraphLayout::new(s, width).enumerate() {
        let height: f32 = lines.iter().map(|line| line.height).sum();
        
        let mut image = GrayImage::from_pixel(
            (width + 2.*margin) as u32,
            (height + 2.*margin) as u32,
            Luma::<u8>{ data: [255] }
        );
        let mut y = margin;
        for line in lines.iter() {
            y += line.height;
            for &(ref word, x) in line.words.iter() {
                word.draw_at(&mut image, (x+margin, y));
            }
        }
        
        image.save(format!("/tmp/layout_{:02}.png", n));
    }
}


