from liblrs_python import Builder, Point, Lrs, SegmentOfTraversal, AnchorOnLrm

def build_lrm() -> Lrs:
    """Helper function that builds an example LRS

    It has the following topology:
        3
        |
    0---1---2

    0-1-2 is lrm_1
    1-3 is lrm_2
    """


    builder = Builder()

    p0 = Point(0, 0)
    p1 = Point(1, 0)
    p2 = Point(2, 0)
    p3 = Point(1, 1)

    builder.add_node("0", p0, {})
    builder.add_node("1", p1, {})
    builder.add_node("2", p2, {})
    builder.add_node("3", p3, {})


    segment1 = SegmentOfTraversal(builder.add_segment("s1", [p0, p1], 0, 1), False)
    segment2 = SegmentOfTraversal(builder.add_segment("s2", [p2, p1], 1, 2), False)
    segment3 = SegmentOfTraversal(builder.add_segment("s1", [p1, p3], 0, 1), False)
    
    traversal1 = builder.add_traversal("traversal1", [segment1, segment2])
    traversal2 = builder.add_traversal("traversal2", [segment3])

    builder.add_lrm("lrm_1", traversal1, [], {})
    builder.add_lrm("lrm_2", traversal2, [], {})

    return builder.build_lrs({})


def test_get_topology():
    lrs = build_lrm()
    nodes = lrs.get_nodes()
    assert len(nodes) == 4

    n0 = lrs.get_node(0)
    assert n0.id == "0"
    assert n0.properties == {}
    assert n0.geometry is not None
    assert n0.geometry.x == 0
    assert n0.geometry.y == 0

    segments = lrs.get_segments()
    assert len(segments) == 3

    segment1 = segments[0]
    assert segment1.id == "s1"
    assert segment1.start_node == 0
    assert segment1.end_node == 1
