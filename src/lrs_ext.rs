//! High level extensions meant for an easy usage
//! Those functions are exposed in wasm-bindings

use geo::{Coord, Point};

use crate::curves::{Curve, SphericalLineStringCurve};
use crate::lrm_scale::{Anchor, LrmScaleMeasure};
use crate::lrs::{self, LrmProjection, LrsBase, LrsError, Properties, TraversalPosition};

type Lrs = lrs::Lrs<SphericalLineStringCurve>;

/// Struct exposed to js.
pub struct ExtLrs {
    /// The linear referencing system
    pub lrs: Lrs,
}

impl ExtLrs {
    /// Load the data.
    pub fn load(data: &[u8]) -> Result<ExtLrs, String> {
        Lrs::from_bytes(data)
            .map(|lrs| Self { lrs })
            .map_err(|err| err.to_string())
    }

    /// How many LRMs compose the LRS.
    pub fn lrm_len(&self) -> usize {
        self.lrs.lrm_len()
    }

    /// Return the geometry of the LRM.
    pub fn get_lrm_geom(&self, index: usize) -> Result<Vec<geo::Coord>, String> {
        let lrm = self.lrs.lrms.get(index).ok_or("Invalid index")?;
        self.lrs
            .get_linestring(lrm.traversal)
            .map_err(|err| err.to_string())
            .map(|linestring| linestring.0)
    }

    /// `id` of the [`LrmScale`].
    pub fn get_lrm_scale_id(&self, lrm_index: usize) -> String {
        self.lrs.lrms[lrm_index].scale.id.clone()
    }

    /// All the [`Anchor`]s of a LRM.
    pub fn get_anchors(&self, lrm_index: usize) -> Vec<Anchor> {
        self.lrs.lrms[lrm_index].scale.anchors.to_vec()
    }

    /// Get the position given a [`LrmScaleMeasure`].
    pub fn resolve(&self, lrm_index: usize, measure: &LrmScaleMeasure) -> Result<Point, LrsError> {
        let lrm = &self.lrs.lrms[lrm_index];
        let curve_position = lrm.scale.locate_point(measure)?.clamp(0., 1.0);

        let traversal_position = TraversalPosition {
            curve_position,
            traversal: lrm.traversal,
        };
        self.lrs.locate_traversal(traversal_position)
    }

    /// Given two [`LrmScaleMeasure`]s, return a range of [`LineString`].
    pub fn resolve_range(
        &self,
        lrm_index: usize,
        from: &LrmScaleMeasure,
        to: &LrmScaleMeasure,
    ) -> Result<Vec<Coord>, String> {
        let lrm = &self.lrs.lrms[lrm_index];
        let scale = &lrm.scale;
        let curve = &self.lrs.traversals[lrm.traversal.0].curve;
        let from = scale
            .locate_point(from)
            .map_err(|e| e.to_string())?
            .clamp(0., 1.);
        let to = scale
            .locate_point(to)
            .map_err(|e| e.to_string())?
            .clamp(0., 1.);

        match curve.sublinestring(from, to) {
            Some(linestring) => Ok(linestring.0),
            None => Err("Could not find sublinestring".to_string()),
        }
    }

    /// [`Properties`] of the lrs
    pub fn lrs_properties(&self) -> &Properties {
        &self.lrs.properties
    }

    /// [`Properties`] for a given lrm
    pub fn lrm_properties(&self, lrm_index: usize) -> &Properties {
        &self.lrs.lrms[lrm_index].properties
    }

    /// [`Properties`] for a given anchor
    pub fn anchor_properties(&self, lrm_index: usize, anchor_index: usize) -> &Properties {
        self.lrs.lrms[lrm_index].scale.anchors[anchor_index].properties()
    }

    /// Projects a [`Point`] on all [`Lrm`] where the [`Point`] is in the bounding box.
    /// The result is sorted by `orthogonal_offset`: the nearest [`Lrm`] to the [`Point`] is the first item.
    pub fn lookup_lrms(&self, point: Point) -> Vec<LrmProjection> {
        self.lrs.lookup_lrms(point)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use geo::{Coord, coord, point};

    use crate::builder::{AnchorOnLrm, Builder, SegmentOfTraversal};
    use crate::{lrs, properties};

    fn build_lrm(builder: &mut Builder, name: &str, coords: &[Coord]) {
        let segment_index = builder.add_segment("name", coords, 0, 1);
        let sot = SegmentOfTraversal {
            segment_index,
            reversed: false,
        };
        let traversal_index = builder.add_traversal(name, &[sot]);
        let start_anchor = AnchorOnLrm {
            anchor_index: builder.add_anchor("start", Some("start"), coords[0], properties!()),
            distance_along_lrm: 0.0,
        };
        let end_anchor = AnchorOnLrm {
            anchor_index: builder.add_anchor(
                "end",
                Some("end"),
                *coords.last().unwrap(),
                properties!(),
            ),
            distance_along_lrm: 1.0,
        };
        builder.add_lrm(
            name,
            traversal_index,
            &[start_anchor, end_anchor],
            properties!(),
        );
    }

    #[test]
    fn test() {
        let mut b = Builder::new();
        build_lrm(&mut b, "lrm1", &[coord! {x:0., y:0.}, coord! {x:2., y:0.}]);
        build_lrm(&mut b, "lrm2", &[coord! {x:0., y:1.}, coord! {x:2., y:1.}]);
        build_lrm(&mut b, "lrm3", &[coord! {x:1., y:1.}, coord! {x:1., y:-1.}]);
        let lrs = b.build_lrs(properties!()).unwrap();

        let nearest0 = lrs.lookup_lrms(point! {x: 1.0001, y:0.0});
        assert_eq!(nearest0.len(), 2);
        assert_eq!(nearest0[0].measure.lrm, lrs::LrmHandle(0));
        assert_eq!(nearest0[1].measure.lrm, lrs::LrmHandle(2));

        let nearest1 = lrs.lookup_lrms(point! {x: 0.5, y:1.0});
        assert_eq!(nearest1.len(), 1);
        assert_eq!(nearest1[0].measure.lrm, lrs::LrmHandle(1));

        let nearest2 = lrs.lookup_lrms(point! {x:1., y:-0.0001});
        assert_eq!(nearest2.len(), 2);
        assert_eq!(nearest2[0].measure.lrm, lrs::LrmHandle(2));
        assert_eq!(nearest2[1].measure.lrm, lrs::LrmHandle(0));

        let nearest3 = lrs.lookup_lrms(point! {x:2.35, y:48.98});
        assert!(nearest3.is_empty());
    }
}
