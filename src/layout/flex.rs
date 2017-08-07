use layout::Flex;
use std::ops::*;
use units::*;
use num::Zero;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlexMeasure {
    pub shrink:     Length,
    pub stretch:    Length,
    pub width:      Length,
    pub height:     Length
}

impl FlexMeasure {
    pub fn fixed(width: Length) -> FlexMeasure {
        FlexMeasure {
            shrink:  width,
            stretch: width,
            width:   width,
            height:  0.0
        }
    }

    /// factor = -1 => self.shrink,
    /// factor =  0 => self.width,
    /// factor = +1 => self.stretch
    pub fn at(&self, factor: Length) -> Length {
        (if factor < 0. {
            (self.width - self.shrink)
        } else {
            (self.stretch - self.width)
        } * factor + self.width)
    }
    
    /// calculate the factor that yields the given length.
    /// Or None when there aint one.
    // m.at(m.factor(w).unwrap()) == w
    pub fn factor(&self, width: Length) -> Option<Length> {
        if width < self.shrink {
            return None;
        }
        
        if width == self.width {
            Some(1.0)
        } else {
            let delta = width - self.width; // d > 0 => stretch, d < 0 => shrink
            let diff = if delta >= 0. {
                self.stretch - self.width
            } else {
                self.width - self.shrink
            };
            Some(delta / diff)
        }
    }
    
    pub fn extend(&mut self, width: Length) {
        if width > self.width {
            self.width = width;
            if width > self.stretch {
                self.stretch = width;
            }
        }
    }
}
impl Add for FlexMeasure {
    type Output = FlexMeasure;
    
    fn add(self, rhs: FlexMeasure) -> FlexMeasure {
        FlexMeasure {
            width:   self.width   + rhs.width,
            stretch: self.stretch + rhs.stretch,
            shrink:  self.shrink  + rhs.shrink,
            height:  self.height.max(rhs.height)
        }
    }
}
impl Zero for FlexMeasure {
    fn zero() -> FlexMeasure {
        FlexMeasure {
            width: 0.,
            stretch: 0.,
            shrink: 0.,
            height: 0.
        }        
    }
    fn is_zero(&self) -> bool {
        self.width == 0. &&
        self.stretch == 0. &&
        self.shrink == 0. &&
        self.height == 0.
    }
}
impl Default for FlexMeasure {
    fn default() -> FlexMeasure {
        FlexMeasure::zero()
    }
}
impl AddAssign for FlexMeasure {
    fn add_assign(&mut self, rhs: FlexMeasure) {
        self.width += rhs.width;
        self.stretch += rhs.stretch;
        self.shrink += rhs.shrink;
        self.height = self.height.max(rhs.height);
    }
}
impl SubAssign for FlexMeasure {
    fn sub_assign(&mut self, rhs: FlexMeasure) {
        self.width -= rhs.width;
        self.stretch -= rhs.stretch;
        self.shrink -= rhs.shrink;
        self.height = self.height.max(rhs.height);
    }
}
impl Mul<f32> for FlexMeasure {
    type Output = FlexMeasure;
    
    fn mul(self, f: f32) -> FlexMeasure {
        FlexMeasure {
            width:      self.width * f,
            stretch:    self.stretch * f,
            shrink:     self.shrink * f,
            height:     self.height
        }
    }
}
impl Flex for FlexMeasure {
    fn measure(&self, _: f32) -> FlexMeasure {
        *self
    }
    
    fn flex(&self, _: f32) -> FlexMeasure {
        FlexMeasure {
            width: self.width,
            shrink: self.shrink,
            stretch: self.stretch,
            height: self.height
        }
    }
}
