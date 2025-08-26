from pathlib import Path
from liblrs_python import Builder, Point, Lrs, SegmentOfTraversal, AnchorOnLrm

def build_lrm() -> Builder:
    """Helper function that builds an example LRS with one lrm"""

    # First we initialize the builder that will help us as we create new data
    builder = Builder()

    # We want to create a segment that has coordinates and nodes at its extremity
    coords = [Point(0, 0), Point(0, 1)]
    segment_id = "a small section of rail"
    start_node = 0
    end_node = 1
    segment_handle = builder.add_segment(segment_id, coords, start_node, end_node)
    
    # Then we build a traversal with that segment
    # Typically, a highway will be composed of many segments between each intersection
    segment_reversed = False
    segment_of_traversal = SegmentOfTraversal(segment_handle, segment_reversed)
    traversal_id = "A very long railroad composed of many segments"
    traversal_handle = builder.add_traversal(traversal_id, [segment_of_traversal])

    # Anchors are reference points with coordinates that will help us to position ourself by measuring a distance from then
    green_anchor = builder.add_anchor(id="Green House", coord=Point(0.01, 0), name="GH", properties={})
    red_anchor = builder.add_anchor(id="Red House", coord=Point(-0.01, 1), name="RH", properties={})

    # We associate those anchors to a traversal by defining a distance along that traversal
    green_on_traversal = AnchorOnLrm(green_anchor, 0)
    red_on_traversal = AnchorOnLrm(red_anchor, 1000)

    # Now we have every thing to create a linear reference model
    builder.add_lrm("My first Lrm", traversal_handle, [green_on_traversal, red_on_traversal], {})

    return builder


def test_build_in_memory():
    builder = build_lrm()
    lrs = builder.build_lrs({})
    assert lrs.lrm_len() == 1

def test_build_to_file(tmp_path: Path):
    path = Path(tmp_path, "my_lrs")
    builder = build_lrm()
    builder.save(path, {})
    with open(path, "rb") as lrs_file:
        lrs = Lrs(lrs_file.read())
        assert lrs.lrm_len() == 1
