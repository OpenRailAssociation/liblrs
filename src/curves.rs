//! This module defines a curve and primitive functions on them
//! Most common manipulations are projecting a point on the curve
//! and otherway round find the coordinates of a point along the curve

use geo::kernels::RobustKernel;
use geo::prelude::*;
use geo::{coord, Line, LineString, Point, Rect};
use thiserror::Error;

/// Errors when manipulating the curves
#[derive(Error, Debug)]
pub enum CurveError {
    /// To be valid, a curve must have at least two distinct points
    #[error("the curve geometry is not valid (at least two distinct points)")]
    InvalidGeometry,
    /// At least a coordinate is non a finite number (NaN, inifinite)
    #[error("the coordinates are not finite")]
    NotFiniteCoordinates,
    /// A considered point is not on the curve
    #[error("the point is not on the curve")]
    NotOnTheCurve,
}

/// A curve is the fundamental building block for an LRM
/// It provides basic primitives to locate/project points on it
/// A curve can be part of a larger curve (e.g. for optimisation purpurses and have better bounding boxes)
pub struct Curve {
    /// When curve might be a piece of a longer curve
    /// then the start_offset allows to know how fare along the longer curve we are
    pub start_offset: usize,
    /// The max distance that is considered of being part of the curve
    /// It is used to compute the bounding box
    pub max_extent: usize,
    /// The coordinates are considered as being planar
    /// All distance and length computations are in units of those coordinates
    pub geom: LineString,
}

impl Curve {
    /// Build a new curve from a LineString
    /// max_extent is the maximum distance that is considered to be “on the curve”
    /// max_extent plays a role in the bounding box
    pub fn new(geom: LineString, max_extent: usize) -> Self {
        Self {
            start_offset: 0,
            max_extent,
            geom,
        }
    }

    /// Splits the LineString into smaller curves of at most `max_len` length
    /// If the initial geometry is invalid, it returns an empty vector
    pub fn new_fragmented(geom: LineString, max_len: usize, max_extent: usize) -> Vec<Curve> {
        let n = (geom.euclidean_length() / max_len as f64).ceil() as usize;
        geom.line_segmentize(n)
            .map(|multi| {
                multi
                    .0
                    .into_iter()
                    .map(|geom| Curve::new(geom, max_extent))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Project the point to the closest position on the curve
    /// Will fail if the curve is invalid (e.g. no points on it)
    /// or if the point is to far away
    /// If the curve is a piece of a larger curve (start_offset > 0)
    /// then the distance_along_curve if from the whole curve, not just the current piece
    pub fn project(&self, point: Point) -> Result<CurveProjection, CurveError> {
        if !self.is_valid() {
            return Err(CurveError::InvalidGeometry);
        }

        match self.geom.line_locate_point(&point) {
            Some(location) => {
                let distance_along_curve =
                    (location * self.geom.euclidean_length()) as usize + self.start_offset;

                let begin = self.geom.coords().next().unwrap();
                let end = self.geom.coords().next_back().unwrap();

                let sign = match RobustKernel::orient2d(point.into(), *end, *begin) {
                    Orientation::Clockwise => 1.,
                    _ => -1.,
                };
                let offset = (point.euclidean_distance(&self.geom) * sign) as isize;

                Ok(CurveProjection {
                    distance_along_curve,
                    offset,
                })
            }
            None => Err(CurveError::NotFiniteCoordinates),
        }
    }

    /// Returns the geographical position of a point on the curve
    /// Will return an error if the CurveProjection is not on this Curve
    pub fn resolve(&self, projection: &CurveProjection) -> Result<Point, CurveError> {
        let fraction = (projection.distance_along_curve as f64 - self.start_offset as f64)
            / self.length() as f64;
        if !(0. ..=1.).contains(&fraction) || fraction.is_nan() {
            Err(CurveError::NotOnTheCurve)
        } else {
            Ok(self.geom.line_interpolate_point(fraction).unwrap())
        }
    }

    /// Bounding box of the curve with a buffer of `max_extent`
    pub fn bbox(&self) -> Rect {
        let bounding_rect = self.geom.bounding_rect().unwrap();
        let max_extent = self.max_extent as f64;
        Rect::new(
            coord! {x: bounding_rect.min().x - max_extent,
                y: bounding_rect.min().y - max_extent,
            },
            coord! {
                x: bounding_rect.max().x + max_extent,
                y: bounding_rect.max().y + max_extent,
            },
        )
    }

    /// The length of the curve
    pub fn length(&self) -> usize {
        self.geom.euclidean_length() as usize
    }

    /// Returns the point where the curve and the segment intersect
    /// If the segment intersects the curve multiple times, an intersection is chosen randomly
    /// When the segment is colinear with the curve it is ignored
    pub fn intersect_segment(&self, segment: Line) -> Option<Point> {
        use geo::line_intersection::line_intersection;
        self.geom
            .lines()
            .flat_map(|curve_line| match line_intersection(segment, curve_line) {
                Some(LineIntersection::SinglePoint {
                    intersection,
                    is_proper: _,
                }) => Some(intersection.into()),
                Some(LineIntersection::Collinear { intersection: _ }) => None,
                None => None,
            })
            .next()
    }

    /// Computes the normal at a given offset on the curve
    /// Will return an error if the curve is invalid or the offset is outside of the curve
    pub fn get_normal(&self, offset: usize) -> Result<Line, CurveError> {
        // We find the point where the normal will start
        let point = self.resolve(&CurveProjection {
            distance_along_curve: offset,
            offset: 0,
        })?;

        // We find the line where the point is located
        // This line will be used to construct the normal:
        // - we translate it so that it starts at `point`
        // - we rotated it by 90°
        // - we scale it in order to be a unit vector
        let line = self
            .geom
            .lines_iter()
            .find(|line| line.contains(&point))
            .ok_or(CurveError::NotFiniteCoordinates)?;

        // We need the length of the length for scaling
        let length = line.euclidean_length();

        // We use the position to know how much to translate so that the vector starts at `point`
        let position = line
            .line_locate_point(&point)
            .ok_or(CurveError::NotFiniteCoordinates)?;
        let dx = line.dx() * position;
        let dy = line.dy() * position;
        dbg!(dx, dy);

        let transform = AffineTransform::translate(dx, dy)
            .scaled(1. / length, 1. / length, line.start)
            .rotated(90., line.start);
        let result = line.affine_transform(&transform);
        Ok(result)
    }

    /// Is the geometry valid
    /// It must have at least two coordinates
    /// If there are exactly two coordinates, they must be different
    pub fn is_valid(&self) -> bool {
        self.geom.coords_count() >= 2 && (self.geom.coords_count() > 2 || !self.geom.is_closed())
    }
}

/// Represents a point in space projected on the curve
pub struct CurveProjection {
    /// How far from the curve start is located the point
    /// If the curve is part of a larger curve, start_offset is strictly positive
    /// and the start_offset will be considered
    pub distance_along_curve: usize,
    /// How far is the point from the curve (euclidian distance)
    /// It is positive if the point is located on the left of the curve
    /// and negative if the point is on the right
    pub offset: isize,
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use geo::line_string;
    use geo::point;

    #[test]
    fn length() {
        let c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);
        assert_eq!(2, c.length());
    }

    #[test]
    fn projection() {
        let mut c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);

        let projected = c.project(point! {x: 1., y: 1.}).unwrap();
        assert_eq!(1, projected.distance_along_curve);
        assert_eq!(1, projected.offset);

        let projected = c.project(point! {x: 1., y: -1.}).unwrap();
        assert_eq!(1, projected.distance_along_curve);
        assert_eq!(-1, projected.offset);

        c.start_offset = 1;
        let projected = c.project(point! {x: 1., y: -1.}).unwrap();
        assert_eq!(2, projected.distance_along_curve);
    }

    #[test]
    fn resolve() {
        let mut c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);

        let mut projection = CurveProjection {
            distance_along_curve: 1,
            offset: 0,
        };
        let p = c.resolve(&projection).unwrap();
        assert_eq!(p.x(), 1.);
        assert_eq!(p.y(), 0.);

        c.start_offset = 1;
        let p = c.resolve(&projection).unwrap();
        assert_eq!(p.x(), 0.);

        projection.distance_along_curve = 4;
        assert!(c.resolve(&projection).is_err());
    }

    #[test]
    fn bbox() {
        let c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);
        let bbox = c.bbox();

        assert_eq!(bbox.min(), coord! {x: -1., y: -1.});
        assert_eq!(bbox.max(), coord! {x: 3., y: 1.});
    }

    #[test]
    fn intersect_segment() {
        // Right angle
        let c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);
        let segment = Line::new(coord! {x: 1., y: 1.}, coord! {x: 1., y: -1.});
        let intersection = c.intersect_segment(segment);
        assert_eq!(intersection, Some(point! {x: 1., y: 0.}));

        // No intersection
        let segment = Line::new(coord! {x: 10., y: 10.}, coord! {x:20., y: 10.});
        assert!(c.intersect_segment(segment).is_none());

        // Colinear
        let segment = Line::new(coord! {x: 0., y:0.,}, coord! {x: 1., y:0.});
        assert!(c.intersect_segment(segment).is_none());

        // Multiple intersection
        let c = Curve::new(
            line_string![(x: 0., y: 0.), (x: 1., y:2.), (x: 2., y: 0.)],
            1,
        );
        let segment = Line::new(coord! {x: 0., y: 1.}, coord! {x: 2., y: 1.});
        assert!(c.intersect_segment(segment).is_some());
    }

    #[test]
    fn fragmented() {
        let c = Curve::new_fragmented(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1, 1);
        assert_eq!(2, c.len());
        assert_eq!(1, c[0].length());
    }

    #[test]
    fn normal() {
        let c = Curve::new(line_string![(x: 0., y: 0.), (x: 2., y:0.)], 1);
        let normal = c.get_normal(1).unwrap();
        assert_eq!(normal.start, coord! {x: 1., y: 0.});
        assert_eq!(normal.end, coord! {x: 1., y: 1.});
    }
}