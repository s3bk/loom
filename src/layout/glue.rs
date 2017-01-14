use std::ops::{BitOr, BitOrAssign};
use std::fmt;

#[derive(Copy, Clone)]
pub enum Glue {
    None,
    Space {
        breaking:   bool,
        scale:      f32
    },
    Newline {
        fill:       bool
    }
}
fn combine_scale(left: f32, right: f32) -> f32 {
    if left > right {
        left
    } else {
        right
    }
}

impl BitOr for Glue {
    type Output = Glue;
    
    fn bitor(self, rhs: Glue) -> Glue {
        use self::Glue::*;
        
        match (self, rhs) {
            // Glue::None wins over anything else
            (None, _) | (_, None) => None,
            
            (Space { breaking: false, .. }, Newline { .. }) |
            (Newline { .. }, Space { breaking: false, .. }) => {
                panic!("Newline and NonBreaking requested");
            },
            
            // NonBreaking wins over Breaking
            (Space { breaking: false, scale: a }, Space { breaking: true,  scale: b }) |
            (Space { breaking: true,  scale: a }, Space { breaking: false, scale: b })
             => Space { breaking: false, scale: combine_scale(a, b) },
            
            // Newline wins over Breaking
            (Newline { fill: a }, Space { breaking: true, .. }) |
            (Space { breaking: true, .. }, Newline { fill: a })
             => Newline { fill: a },
            
            (Space { breaking: true, scale: a }, Space { breaking: true,  scale: b })
             => Space { breaking: true, scale: combine_scale(a, b) },
             
            (Space { breaking: false, scale: a }, Space { breaking: false,  scale: b })
             => Space { breaking: false, scale: combine_scale(a, b) },
             
            (Newline { fill: a }, Newline { fill: b })
             => Newline { fill: a | b }
        }
    }
}
impl BitOrAssign for Glue {
    fn bitor_assign(&mut self, rhs: Glue) {
        *self = *self | rhs;
    }
}

impl Glue {
    pub fn space() -> Glue {
        Glue::Space { breaking: true, scale: 1.0 }
    }
    pub fn nbspace() -> Glue {
        Glue::Space { breaking: false, scale: 1.0 }
    }
    pub fn newline() -> Glue {
        Glue::Newline { fill: false }
    }
    pub fn hfill() -> Glue {
        Glue::Newline { fill: true }
    }
    pub fn any() -> Glue {
        Glue::Space { breaking: true, scale: 1.0 }
    }
}

impl fmt::Display for Glue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Glue::None => Ok(()),
            Glue::Space { breaking: true, .. } => write!(f, "␣"),
            Glue::Space { breaking: false, .. } => write!(f, "~"),
            Glue::Newline { fill: _ } => write!(f, "␤")
        }
    }
}

