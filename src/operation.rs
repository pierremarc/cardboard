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

pub fn paint_op_debug(op: &Operation, style: &StyleCollection, context: &Context) {
    match op {
        Operation::Begin => {
            println!("NEW PATH");
            context.new_path()
        }
        Operation::Close => {
            println!("CLOSE PATH");
            context.close_path()
        }
        Operation::Move(p) => {
            println!("MOVE {} {}", p.x, p.y);
            context.move_to(p.x, p.y)
        }
        Operation::Line(p) => {
            println!("LINE {} {}", p.x, p.y);
            context.line_to(p.x, p.y)
        }
        Operation::Paint(li, si) => style.get_for(li, si).map_or((), |s| {
            s.fillColor.map(|color| {
                println!("FILL {} {} {}", color.red, color.green, color.blue,);
                context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                context.fill_preserve();
            });

            s.strokeColor.map(|color| {
                println!(
                    "STROKE {} {} {} {}",
                    s.strokeWidth, color.red, color.green, color.blue,
                );
                context.set_line_width(s.strokeWidth);
                context.set_source_rgba(color.red, color.green, color.blue, color.alpha);
                context.stroke();
            });
        }),
    }
}
