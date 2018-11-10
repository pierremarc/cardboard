use cairo::Context;
use lingua::Point2D;
use style::{StyleCollection, StyleGetter};

pub enum Operation {
    Move(Point2D),
    Line(Point2D),
    Begin,
    Close,
    Paint(usize, usize),
}

pub type OpList = Vec<Operation>;

pub fn paint_op(op: &Operation, style: &StyleCollection, context: &Context) {
    match op {
        Operation::Begin => context.new_path(),
        Operation::Close => context.close_path(),
        Operation::Move(p) => context.move_to(p.x, p.y),
        Operation::Line(p) => context.line_to(p.x, p.y),
        Operation::Paint(li, si) => style.get_for(li, si).map_or((), |s| {
            s.fillColor.map(|color| {
                context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                context.fill_preserve();
            });

            s.strokeColor.map(|color| {
                context.set_line_width(s.strokeWidth);
                context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                context.stroke();
            });
        }),
    }
}
