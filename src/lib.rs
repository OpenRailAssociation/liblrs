#[allow(unused_imports)]
#[allow(clippy::all)]
#[allow(dead_code)]
#[allow(mismatched_lifetime_syntaxes)]
#[allow(unsafe_op_in_unsafe_fn)]
#[rustfmt::skip]
mod lrs_generated;

#[deny(missing_docs)]
mod osm_helpers;

#[deny(missing_docs)]
pub mod curves;
#[deny(missing_docs)]
pub mod lrm_scale;
#[deny(missing_docs)]
pub mod lrs;
#[deny(missing_docs)]
pub mod lrs_ext;

#[deny(missing_docs)]
pub mod builder;

pub trait DataIssueReporter {
    fn report_ignoring_traversal_edges(
        &mut self,
        traversal_ref: &str,
        ignored_count: usize,
        total_count: usize,
        first_node: i64,
        last_node: i64,
    );
}

pub struct LoggingDataIssueReporter;

impl DataIssueReporter for LoggingDataIssueReporter {
    fn report_ignoring_traversal_edges(
        &mut self,
        traversal_ref: &str,
        ignored_count: usize,
        total_count: usize,
        first_node: i64,
        last_node: i64,
    ) {
        println!(
            "[WARN] on traversal {traversal_ref}, ignoring {ignored_count} edges out of {total_count}. Sorted from {first_node} to {last_node}"
        );
    }
}

impl DataIssueReporter for () {
    fn report_ignoring_traversal_edges(
        &mut self,
        _traversal_ref: &str,
        _ignored_count: usize,
        _total_count: usize,
        _first_node: i64,
        _last_node: i64,
    ) {
    }
}

#[test]
fn read_and_write_lrs() {
    use builder::*;
    use curves::SphericalLineStringCurve;
    use geo::Coord;

    let mut builder = Builder::new();
    let anchor_index = builder.add_anchor(
        "Ancre",
        Some("12"),
        Coord { x: 0., y: 0. },
        properties!("some key" => "some value"),
    );
    let start_node = builder.add_node("a", Coord { x: 0., y: 0. }, properties!());
    let end_node = builder.add_node("b", Coord { x: 1., y: 1. }, properties!());
    let segment_geometry = &[Coord { x: 0., y: 0. }, Coord { x: 1., y: 1. }];
    let segment = SegmentOfTraversal {
        segment_index: builder.add_segment("segment", segment_geometry, start_node, end_node),
        reversed: false,
    };
    let traversal = builder.add_traversal("traversal", &[segment]);
    let anchor_on_lrm = AnchorOnLrm {
        anchor_index,
        distance_along_lrm: 12.0,
    };
    builder.add_lrm("lrm", traversal, &[anchor_on_lrm], properties!());

    let buffer = builder.build_data(properties!("source" => "example"));
    let lrs = lrs::Lrs::<SphericalLineStringCurve>::from_bytes(buffer).unwrap();
    match &lrs.lrms[0].scale.anchors[0] {
        lrm_scale::Anchor::Named(anchor) => assert_eq!(anchor.name, "12"),
        _ => unreachable!(),
    }
}
