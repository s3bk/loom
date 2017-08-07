struct Color {}
enum Slanted {
    Normal,
    Italic,
    Oblique
}
enum FontType {
    Serif,
    SansSerif,
    Monospace
}
struct Size {}

trait Field {
    fn field_mut(&mut self, name: &str) -> Option<&mut Field>;
    fn field(&self, name: &str) -> Option<&Field>;
    fn assign(&mut self, value: &Field) -> Result;
    fn call(&self) -> Option<Box<Field>>;
}

// x.color = Color("green")..mix(parent.color)
// get_mut("x").field_mut("color").assign(Color::from("green").mix(get("parent").

trait VectorFont {
    fn color(&mut self) -> &mut Color;
}

Text based
  - font attributes
      - bold
      - italic
      - color {mix}
  - font style
      - monospace | sans-serif | serif
  
Vector based
  - Font
      - size {mix, min, max}
      - weight [thin .. fat] {mix, min, max}
      - variant [oblique | italic | bold]
  - Space
      - margin {mix, min, max}
      - vertical space {mix, min, max}
  - Color
      - brightness {mix, min, max}
      - hue {mix}
      - saturation {mix, min, max}
