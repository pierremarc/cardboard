use lingua::PlaneList;
use lingua::Point;
use ordered_float::OrderedFloat;
use std::cmp;

pub struct BBox {
    minx: f64,
    miny: f64,
    minz: f64,
    maxx: f64,
    maxy: f64,
    maxz: f64,
}

impl BBox {
    pub fn from_planes(pl: &PlaneList) -> BBox {
        let mut minx = OrderedFloat(std::f64::MAX);
        let mut miny = OrderedFloat(std::f64::MAX);
        let mut minz = OrderedFloat(std::f64::MAX);
        let mut maxx = OrderedFloat(std::f64::MIN);
        let mut maxy = OrderedFloat(std::f64::MIN);
        let mut maxz = OrderedFloat(std::f64::MIN);

        pl.iter().for_each(|plane| {
            plane.points.iter().for_each(|pt| {
                minx = cmp::min(minx, OrderedFloat(pt.x));
                miny = cmp::min(miny, OrderedFloat(pt.y));
                minz = cmp::min(minz, OrderedFloat(pt.z));
                maxx = cmp::max(maxx, OrderedFloat(pt.x));
                maxy = cmp::max(maxy, OrderedFloat(pt.y));
                maxz = cmp::max(maxz, OrderedFloat(pt.z));
            })
        });
        BBox {
            minx: minx.into_inner(),
            miny: miny.into_inner(),
            minz: minz.into_inner(),
            maxx: maxx.into_inner(),
            maxy: maxy.into_inner(),
            maxz: maxz.into_inner(),
        }
    }

    pub fn center(&self) -> Point {
        Point::new(
            self.minx + ((self.maxx - self.minx) / 2.0),
            self.miny + ((self.maxy - self.miny) / 2.0),
            self.minz + ((self.maxz - self.minz) / 2.0),
        )
    }

    pub fn width(&self) -> f64 {
        self.maxx - self.minx
    }

    pub fn height(&self) -> f64 {
        self.maxz
    }

    pub fn top_left_near(&self) -> Point {
        Point::new(self.minx, self.miny, self.maxz)
    }
}
