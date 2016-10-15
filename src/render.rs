use layout;
use document;
use environment::{Environment, prepare_environment};
use layout::LayoutNode;
use image::{GrayImage, Luma};
use blocks::RootNode;
use std::path::Path;

#[test]
fn test_render() {
    render_file(Path::new("doc/reference.yarn"), 600.)
    .save("/tmp/layout.png").unwrap();
}

    
fn render(node: &LayoutNode, width: f32) -> GrayImage {
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

pub fn render_data(data: &str, width: f32) -> GrayImage {
    use environment::prepare_environment;
    
    let root = RootNode{};
    let mut env = Environment::new();
    prepare_environment(&mut env);
    let root = RootNode::parse(data);
    render(&root, width)
}
pub fn render_file(path: &Path, width: f32) -> GrayImage {
    use std::fs::File;
    use std::io::Read;
    
    let mut f = File::open(path).expect("no such file");
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    render_data(&s, width)
}

