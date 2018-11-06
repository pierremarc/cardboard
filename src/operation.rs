use lingua::Point2D;

pub enum Operation {
    Move(Point2D),
    Line(Point2D),
    Begin,
    Close,
    Paint(usize),
}

pub type OpList = Vec<Operation>;
