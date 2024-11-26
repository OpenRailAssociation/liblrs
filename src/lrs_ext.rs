//! High level extensions meant for an easy usage
//! Those functions are exposed in wasm-bindings

use geo::{Coord, Point};

use crate::curves::{Curve, PlanarLineStringCurve, SphericalLineStringCurve};
use crate::lrm_scale::LrmScaleMeasure;
use crate::lrm_scale::{Anchor, LrmScaleError};
use crate::lrs::{Lrm, LrmHandle, LrmProjection, Lrs, LrsError};
use crate::lrs::{LrsBase, TraversalPosition};

/// Struct exposed to js.
pub enum ExtLrs {
    /// LRS with spherical coordinates.
    Spherical(Lrs<SphericalLineStringCurve>),
    /// LRS with planar coordinates.
    Planar(Lrs<PlanarLineStringCurve>),
}

impl ExtLrs {
    /// Load the data.
    pub fn load(data: &[u8], planar: bool) -> Result<ExtLrs, String> {
        if planar {
            Lrs::<PlanarLineStringCurve>::from_bytes(data).map(ExtLrs::Planar)
        } else {
            Lrs::<SphericalLineStringCurve>::from_bytes(data).map(ExtLrs::Spherical)
        }
        .map_err(|err| err.to_string())
    }

    /// How many LRMs compose the LRS.
    pub fn lrm_len(&self) -> usize {
        match self {
            ExtLrs::Spherical(lrs) => lrs.lrm_len(),
            ExtLrs::Planar(lrs) => lrs.lrm_len(),
        }
    }

    /// Given a ID returns the corresponding lrs index (or None if not found)
    pub fn find_lrm(&self, lrm_id: &str) -> Option<usize> {
        match self {
            ExtLrs::Spherical(lrs) => lrs.get_lrm(lrm_id).map(|handle| handle.0),
            ExtLrs::Planar(lrs) => lrs.get_lrm(lrm_id).map(|handle| handle.0),
        }
    }

    fn get_lrm(&self, index: usize) -> &Lrm {
        match self {
            ExtLrs::Spherical(lrs) => &lrs.lrms[index],
            ExtLrs::Planar(lrs) => &lrs.lrms[index],
        }
    }

    /// Return the geometry of the LRM.
    pub fn get_lrm_geom(&self, index: usize) -> Result<Vec<geo::Coord>, String> {
        let lrm = self.get_lrm(index);
        match self {
            ExtLrs::Spherical(lrs) => lrs.get_linestring(lrm.reference_traversal),
            ExtLrs::Planar(lrs) => lrs.get_linestring(lrm.reference_traversal),
        }
        .map_err(|err| err.to_string())
        .map(|linestring| linestring.0)
    }

    /// `id` of the [`LrmScale`].
    pub fn get_lrm_scale_id(&self, index: usize) -> String {
        self.get_lrm(index).scale.id.clone()
    }

    /// All the [`Anchor`]s of a LRM.
    pub fn get_anchors(&self, lrm_index: usize) -> Vec<Anchor> {
        self.get_lrm(lrm_index).scale.anchors.to_vec()
    }

    /// Get the position given a [`LrmScaleMeasure`].
    pub fn resolve(&self, lrm_index: usize, measure: &LrmScaleMeasure) -> Result<Point, LrsError> {
        let lrm = self.get_lrm(lrm_index);
        let curve_position = lrm.scale.locate_point(measure)?.clamp(0., 1.0);

        let traversal_position = TraversalPosition {
            curve_position,
            traversal: lrm.reference_traversal,
        };
        match self {
            ExtLrs::Spherical(lrs) => lrs.locate_traversal(traversal_position),
            ExtLrs::Planar(lrs) => lrs.locate_traversal(traversal_position),
        }
    }

    /// Given two [`LrmScaleMeasure`]s, return a range of [`LineString`].
    pub fn resolve_range(
        &self,
        lrm_index: usize,
        from: &LrmScaleMeasure,
        to: &LrmScaleMeasure,
    ) -> Result<Vec<Coord>, String> {
        let lrm = self.get_lrm(lrm_index);
        let scale = &lrm.scale;
        let from = scale
            .locate_point(from)
            .map_err(|e| e.to_string())?
            .clamp(0., 1.);
        let to = scale
            .locate_point(to)
            .map_err(|e| e.to_string())?
            .clamp(0., 1.);

        let sublinestring = match self {
            ExtLrs::Spherical(lrs) => lrs.traversals[lrm.reference_traversal.0]
                .curve
                .sublinestring(from, to),
            ExtLrs::Planar(lrs) => lrs.traversals[lrm.reference_traversal.0]
                .curve
                .sublinestring(from, to),
        };

        match sublinestring {
            Some(linestring) => Ok(linestring.0),
            None => Err("Could not find sublinestring".to_string()),
        }
    }

    /// Given a point, return the [`LrmProjection`]s.
    pub fn lookup(&self, point: Point, lrm_handle: LrmHandle) -> Vec<LrmProjection> {
        match self {
            ExtLrs::Spherical(lrs) => lrs.lookup(point, lrm_handle),
            ExtLrs::Planar(lrs) => lrs.lookup(point, lrm_handle),
        }
    }

    /// Get the positon along the curve given a [`LrmScaleMeasure`]
    /// The value will be between 0.0 and 1.0, both included
    pub fn locate_point(
        &self,
        lrm_index: usize,
        measure: &LrmScaleMeasure,
    ) -> Result<f64, LrmScaleError> {
        let lrm = self.get_lrm(lrm_index);
        lrm.scale.locate_point(measure)
    }
}
