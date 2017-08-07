use output::Output;

#[derive(Debug)]
pub struct Style<O: Output> {
    pub font: O::Font,
    pub font_size: f32,
    pub leading: f32,
    pub par_indent: f32
}
impl<O: Output> Style<O> {
    pub fn font(&self) -> &O::Font {
        &self.font
    }
}
