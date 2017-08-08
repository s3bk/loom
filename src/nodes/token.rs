use nodes::prelude::*;

pub enum Token {
    HFill,
    QuadSpace,
    Other(IString)
}
impl Token {
    pub fn new(s: IString) -> Token {
        match &*s {
            "hfill" => Token::HFill,
            "quad" => Token::QuadSpace,
            _ => Token::Other(s)
        }
    }
}
impl Node for Token {
    fn layout(&self, _env: LayoutChain, w: &mut Writer) {
        match *self {
            Token::HFill => {
                w.promote(Glue::hfill());
            },
            Token::QuadSpace => {
                w.promote(Glue::Space { breaking: true, scale: 4.0 });
            },
            Token::Other(ref s) => {
                w.word(Atom {
                    text:   &s,
                    left:   Glue::None,
                    right:  Glue::space()
                });
            }
        }
    }
}
