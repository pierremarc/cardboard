use camera::Camera;
use capture::Capture;
use geom::deg_to_rad;
use geom::vertical_axis;
use nalgebra as na;

fn match_mod<F0, F1>(kmod: sdl2::keyboard::Mod, naked: F0, controled: F1) -> Option<Camera>
where
    F0: FnOnce() -> Camera,
    F1: FnOnce() -> Camera,
    // F2: FnOnce() -> Camera,
{
    let ctrl_mod: sdl2::keyboard::Mod = sdl2::keyboard::LCTRLMOD; //& sdl2::keyboard::RCTRLMOD;
                                                                  // let shift_mod: sdl2::keyboard::Mod = sdl2::keyboard::LSHIFTMOD; //& sdl2::keyboard::RSHIFTMOD;
    if kmod.intersects(ctrl_mod) {
        println!("CTRL");
        Some(controled())
    // } else if kmod.intersects(shift_mod) {
    //     println!("SHIFT");
    //     shifted()
    } else {
        println!("NAKED");
        Some(naked())
    }
}

static CAMP_STEP: f64 = 1.2 * 2.0;
static CAM_STEP_ROT: f64 = 0.0174533 * 2.0;

// fn app<A, T, F: FnOnce(A) -> T>(f: F, a: A) -> T {
//     f(a)
// }

pub enum PreAction {
    Reset,
    Capture,
    Follow,
    Save,
    PrintCam,
}

pub fn handle_key_event_pre(
    key: Option<sdl2::keyboard::Keycode>,
    kmod: sdl2::keyboard::Mod,
) -> Option<PreAction> {
    key.and_then(|code| match code {
        sdl2::keyboard::Keycode::R => Some(PreAction::Reset),
        sdl2::keyboard::Keycode::C => Some(PreAction::Capture),
        sdl2::keyboard::Keycode::S => Some(PreAction::Save),
        sdl2::keyboard::Keycode::F => Some(PreAction::Follow),
        sdl2::keyboard::Keycode::P => Some(PreAction::PrintCam),
        _ => None,
    })
}

pub fn handle_key_event(
    key: Option<sdl2::keyboard::Keycode>,
    kmod: sdl2::keyboard::Mod,
    cam: &Camera,
) -> Option<Camera> {
    key.and_then(|code| {
        match code {
            // naked  => camera
            // contol => eye
            // shift  => target
            sdl2::keyboard::Keycode::Left => match_mod(
                kmod,
                || cam.move_cam(cam.side_mov(CAMP_STEP)),
                || cam.rotate_eye(vertical_axis(), CAM_STEP_ROT),
            ),
            sdl2::keyboard::Keycode::Right => match_mod(
                kmod,
                || cam.move_cam(cam.side_mov(-CAMP_STEP)),
                || cam.rotate_eye(vertical_axis(), -CAM_STEP_ROT),
            ),
            sdl2::keyboard::Keycode::Up => match_mod(
                kmod,
                || cam.move_eye(cam.axis_mov(-CAMP_STEP)),
                || cam.rotate_eye(cam.get_horizontal_axis(), -CAM_STEP_ROT),
            ),
            sdl2::keyboard::Keycode::Down => match_mod(
                kmod,
                || cam.move_eye(cam.axis_mov(CAMP_STEP)),
                || cam.rotate_eye(cam.get_horizontal_axis(), CAM_STEP_ROT),
            ),
            _ => None,
        }
    })
}

pub fn handle_motion_event(xrel: i32, yrel: i32, cam: &Camera) -> Option<Camera> {
    let ox = f64::from(xrel);
    let oy = f64::from(yrel);

    let horizontal_axis = cam.get_horizontal_axis();
    let vertical_axis = na::Unit::new_normalize(na::Vector3::new(0.0, 0.0, 1.0));
    let tr = na::Translation3::new(cam.eye.x, cam.eye.y, cam.eye.z).to_homogeneous();
    let itr = na::Translation3::new(-cam.eye.x, -cam.eye.y, -cam.eye.z).to_homogeneous();
    let hmat = na::Matrix4::from_axis_angle(&horizontal_axis, deg_to_rad(oy));
    let vmat = na::Matrix4::from_axis_angle(&vertical_axis, deg_to_rad(ox));
    let op = match (xrel, yrel) {
        (0, 0) => na::Matrix4::identity(),
        (0, _) => tr * vmat * itr,
        (_, 0) => tr * hmat * itr,
        (_, _) => tr * vmat * hmat * itr,
    };
    // println!("{:?} {:?}", horizontal_axis, vertical_axis);

    // println!("handle_motion_event {} {}", ox, oy);
    // println!("before {}", cam.target);
    // println!("after {}", op.transform_point(&cam.target));

    Some(cam.move_target(op))
}

pub fn handle_wheel_event(y: i32, cam: &Camera) -> Option<Camera> {
    let oy = f64::from(y);

    Some(cam.move_target(cam.axis_mov(CAMP_STEP * oy)))
}
