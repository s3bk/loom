trait AreaGen {
    type Area;
    
    fn initial(&self) -> Self::Area;
    fn advance(&mut self, a: Self::Area) -> Self::Area;
    fn width(&self, a: &InitialsState) -> Rect;
}

struct InitialsState {
    first:  bool
}

struct InitialsGen {
    total_width:    Size,
    initials_width: Size,
}

impl AreaGen for InitialsGen {
    type Area = InitialsState
    
    fn initial(&self) -> InitialsState {
        InitialsState {
            first:  true
        }
    }
    
    fn advance(&mut self, a: InitialsState) -> InitialsState {
        InitialsState {
            first:  false
        }
    }
    
    fn width(&self, a: &InitialsState) -> Rect {
        match a.first {
            true => self.total_width - initials_width,
            false => self.total_width
        }
    }
}
