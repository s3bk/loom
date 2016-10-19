#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate loom;
extern crate image;

use slog::{Logger, DrainExt};
use slog_term::streamer;
use loom::io::IoMachine;
use std::path::Path;
use loom::layout::ParagraphLayout;
use image::{GrayImage, Luma};

fn prepare(log_root: Logger) -> IoMachine {
    debug!(log_root, "creating IoMachine");
    let mut io = IoMachine::new(log_root, None);
    
    io.load_yarn(&Path::new("doc/reference.yarn"));
    
    io
}
fn render(log_root: Logger, io: IoMachine) {
    let width = 600.0;
    info!(log_root, "creating layout");
    if let Some(layout) = io.layout() {
        debug!(log_root, "flattening layout");
        let mut p = ParagraphLayout::new(layout.into(), width);
        
        info!(log_root, "calculating layout");
        let lines = p.run();
    
        let margin_v = 10.0;
        let margin_h = 10.0;
        
        let height: f32 = lines.iter().map(|l| l.height).sum();
        let mut image = GrayImage::from_pixel(
            (width + 2. * margin_h) as u32,
            (height + 2. * margin_v) as u32,
            Luma { data: [255u8] }
        );
        
        info!(log_root, "rendering");
        
        let mut y = margin_v;
        for line in lines.iter() {
            y += line.height;
            for &(ref word, x) in line.words.iter() {
                word.draw_at(&mut image, (x+margin_h, y));
            }
        }
        
        info!(log_root, "saving");
        image.save("/tmp/test.png");
    }
}


#[test]
fn test_render() {
    let log_root = Logger::root(streamer().build().fuse(), o!("test" => "test_io"));
    let io = prepare(log_root.clone());
    render(log_root, io);
}
