use lingua::Point;
use nalgebra as na;

pub fn cross(a: &Point, b: &Point) -> na::Vector3<f64> {
    let cx = a.y * b.z - a.z * b.y;
    let cy = a.z * b.x - a.x * b.z;
    let cz = a.x * b.y - a.y * b.x;

    na::Vector3::new(cx, cy, cz)
}

pub fn cross_norm(a: &Point, b: &Point) -> na::Unit<na::Vector3<f64>> {
    na::Unit::new_normalize(cross(a, b))
}

pub fn deg_to_rad(a: f64) -> f64 {
    a * std::f64::consts::PI / 180.0
}

pub fn vertical_axis() -> na::Unit<na::Vector3<f64>> {
    na::Unit::new_normalize(na::Vector3::new(0.0, 0.0, 1.0))
}

pub fn transform2d(
    aligned_point3d: &Point,
    corrective: &na::Matrix3<f64>,
    scale: f64,
    tr: &na::Matrix3<f64>,
) -> na::Point2<f64> {
    let aligned_point = na::Point2::new(aligned_point3d.x, aligned_point3d.y);
    let rotated = corrective.transform_point(&aligned_point);
    let scaled = rotated * scale;
    tr.transform_point(&scaled)
}

#[cfg(test)]
mod tests {
    use geom::cross;
    use lingua::Point;
    use nalgebra as na;
    #[test]
    fn cross_product() {
        let a = Point::new(1.0, 2.0, 3.0);
        let b = Point::new(1.0, 5.0, 7.0);
        let c = cross(&a, &b);
        assert_eq!(c, na::Vector3::new(-1.0, -4.0, 3.0))
    }
}
