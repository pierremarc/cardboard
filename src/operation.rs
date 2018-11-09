use lingua::Point2D;

pub enum Operation {
    Move(Point2D),
    Line(Point2D),
    Begin,
    Close,
    Paint(usize, usize),
}

pub type OpList = Vec<Operation>;
