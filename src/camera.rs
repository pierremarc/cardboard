use geom::cross_norm;
use lingua::Point;
use nalgebra as na;
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub eye: Point,
    pub target: Point,
}

impl Camera {
    pub fn new(eye: Point, target: Point) -> Camera {
        Camera { eye, target }
    }

    pub fn get_horizontal_axis(&self) -> na::Unit<na::Vector3<f64>> {
        let pt0 = Point::new(
            self.eye.x - self.target.x,
            self.eye.y - self.target.y,
            self.eye.z,
        );
        let pt1 = Point::new(
            self.eye.x - self.target.x,
            self.eye.y - self.target.y,
            self.target.z,
        );

        cross_norm(&pt0, &pt1)
    }

    pub fn move_cam(&self, mt: na::Matrix4<f64>) -> Camera {
        Camera {
            eye: mt.transform_point(&self.eye),
            target: mt.transform_point(&self.target),
        }
    }

    pub fn move_eye(&self, mt: na::Matrix4<f64>) -> Camera {
        Camera {
            eye: mt.transform_point(&self.eye),
            target: self.target.clone(),
        }
    }

    pub fn move_target(&self, mt: na::Matrix4<f64>) -> Camera {
        Camera {
            eye: self.eye.clone(),
            target: mt.transform_point(&self.target),
        }
    }

    pub fn rotate_eye(&self, axis: na::Unit<na::Vector3<f64>>, angle: f64) -> Camera {
        let tr =
            na::Translation3::new(self.target.x, self.target.y, self.target.z).to_homogeneous();
        let itr =
            na::Translation3::new(-self.target.x, -self.target.y, -self.target.z).to_homogeneous();
        let mat = na::Matrix4::from_axis_angle(&axis, angle);
        self.move_eye(tr * mat * itr)
    }

    pub fn side_mov(&self, step: f64) -> na::Matrix4<f64> {
        let axis = self.get_horizontal_axis();
        let m = axis.unwrap() * step;
        let tr = na::Translation3::from(m);

        tr.to_homogeneous()
    }

    pub fn axis_mov(&self, step: f64) -> na::Matrix4<f64> {
        let axis = na::Unit::new_normalize(self.target - self.eye);
        let m = axis.unwrap() * step;
        let tr = na::Translation3::from(m);
        tr.to_homogeneous()
    }
}

impl fmt::Display for Camera {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Camera {} {} {} {} {} {}",
            self.eye.x, self.eye.y, self.eye.z, self.target.x, self.target.y, self.target.z,
        )
    }
}
