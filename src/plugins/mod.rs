
/// Plugin architecture
///
/// We do not want "spooky action at a distance" (Spukhafte Fernwirkung) (A. Einstein)
///
/// Plugin::draw() -> VectorBackend::draw(Primitive::Variant) -> Primite::draw
pub trait Plugin : Flex {
    fn min_size(&self, pagewidth) -> Size;
    
    // covered by Flex
    // fn height_for_width(&self, width: Distance);
    
}

pub type Real = f64;

enum SampleHint {
    Continuous,
}  

pub enum Bound {
    Open(Real),
    Closed(Real)
};

pub struct Domain(Bound, Bound);
impl Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Domain(Bound::Open(low),   Bound::Open(high)) =>   write!("({}, {})", low, high),
            &Domain(Bound::Open(low),   Bound::Closed(high)) => write!("({}, {}]", low, high),
            &Domain(Bound::Closed(low), Bound::Open(high)) =>   write!("[{}, {})", low, high),
            &Domain(Bound::Closed(low), Bound::Closed(high)) => write!("[{}, {}]", low, high)
        }
    }
}

pub trait Sampler {
    fn sample(&self, &mut rand::ThreadRng) -> Point;
    fn hints(&self, Range) -> Option<SampleHint> {
        None
    }
    fn domain(&self) -> (Real, Real);
}

pub enum Primitive {
    Point(Point),
    Line(Point, Point),
    Function(&Sampler)
}

/// Implemented by every Vector Output
pub trait VectorBackend {
    fn line(&mut self, a: Point, b: Point);
    //fn bezir3(a: Point, b: Point, c: Point, d: Point)
    fn curve(&Fn)
}

pub trait VectorDraw {
    fn draw(&self, &mut VectorBackend);
}
