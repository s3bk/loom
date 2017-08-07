use canvas::Canvas;
use rand;
use rand::distributions::{IndependentSample, Range as Uniform};
use nalgebra::{Vector2, Point2};
use num::{Float, cast};
use std::ops::Range;
use std::rc::Rc;
use std::convert::{From, Into};


pub struct Parametric<F: Fn(N) -> Point2>  {
    f:          F
    range:      Uniform<N>,
    samples:    usize
}
impl Sampler for Parametric {
    fn sample(&self, rng: &mut rand::ThreadRng) -> Point {
        let c = self.range.ind_sample(rng);
        (self.f)(c)
    }
    fn domain(&self) -> Domain {
        self.domain
    }
}

pub struct Figure {
    domain: (Range<Real>, Range<Real>),
    items:  Vec<Box<Parametric>>,
    ratio:  Scale
}
impl Flex for Figure {
}
impl Object for Figure {
    fn measure(&self, primary: Length) -> FlexMeasure {
        FlexMeasure {
            shrink:     primary,
            stretch:    primary,
            width:      primary,
            height:     primary * self.ratio
        }
    }
    fn show(&self, out: &mut Output) {
    
    }
    fn glue(&self) -> (Glue, Glue) {
        (Glue::newline(), Glue::newline())
    }
}

impl<'a> Figure<'a> {
    pub fn new(x: Range<N>, y: Range<N>) -> Figure<'a> {
        Figure {
            domain: Vector2::new(x, y),
            items: Vec::new()
        }
    }

    pub fn xy<F>(&mut self, f: F, samples: usize)
    where F: Fn(Real) -> Real
    {
        let range = self.range.0;
        self.items.push(
            Box::new(Parametric {
                f:          |x| (x, f(x)),
                range:      Uniform::new(range.start, range.end),
                domain:     Domain(Bound::Closed(range.start), Bound::Open(range.end))
            })
        );
        self
    }
    
    pub fn parametric<F>(&mut self, f: F, range: Range<Real>, samples: usize)
    where F: Fn(Real) -> (Real, Real) 
    {
        self.items.push(
            Box::new(Parametric {
                f:          f,
                range:      Uniform::new(range.start, range.end),
                
            })
        );
        self
    }
    }
}

#[test]
fn test_plot() {
    Figure::new(-5.0 .. 5.0, -5.0 .. 5.0)
        .add(&XY::new(Box::new(|x: f64| (1.0/x).sin()), -5.0 .. 5.0), 10_000);
}
 
