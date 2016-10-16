use layout;
use document;
use environment::{Environment, prepare_environment};
use layout::LayoutNode;
use image::{GrayImage, Luma};
use blocks::RootNode;
use std::path::Path;
    
pub fn render(node: &LayoutNode, width: f32) -> GrayImage {
    use std::time::SystemTime;
    
    fn m(label: &str, t0: SystemTime, t1: SystemTime) {
        let d = t1.duration_since(t0).unwrap();
        println!("{} {:01}.{:09}s", label, d.as_secs(), d.subsec_nanos());
    }
    
    let t2 = SystemTime::now();
    let margin_v = 10.0;
    let margin_h = 10.0;
    
    let lines = layout::ParagraphLayout::new(node.into(), width).run();
    let height: f32 = lines.iter().map(|l| l.height).sum();
    let mut image = GrayImage::from_pixel(
        (width + 2. * margin_h) as u32,
        (height + 2. * margin_v) as u32,
        Luma { data: [255u8] }
    );
    
    let t3 = SystemTime::now();
    m("layout:     ", t2, t3);
    
    let mut y = margin_v;
    for line in lines.iter() {
        y += line.height;
        for &(ref word, x) in line.words.iter() {
            word.draw_at(&mut image, (x+margin_h, y));
        }
    }
    let t4 = SystemTime::now();
    m("drawing:    ", t3, t4);
    
    image
}


