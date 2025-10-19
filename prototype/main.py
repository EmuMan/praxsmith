from praxsmth.graph import WorldGraph, Thing, RelationType


def test_query_exists(
    world_graph: WorldGraph, name: str, sentence: str, expected: bool
):
    print(f"Testing if {name} (expect {expected}):", end=" ")
    actual = world_graph.query_exists(sentence)
    print(actual)
    if actual != expected:
        print(f"Error: Expected {expected}, got {actual}: {sentence}")


def main():
    world_graph = WorldGraph()

    world_graph.add_relation_type(
        RelationType(
            id="contains",
            forward_id="contains",
            forward_name="contains",
            forward_exclusive=False,
            backward_id="in",
            backward_name="is in",
            backward_exclusive=True,
            parameters=[],
        )
    )
    world_graph.add_relation_type(
        RelationType(
            id="likes",
            forward_id="likes",
            forward_name="likes",
            forward_exclusive=False,
            backward_id="liked_by",
            backward_name="is liked by",
            backward_exclusive=False,
            parameters=[],
        )
    )

    world_graph.add_thing(
        Thing(
            id="john",
            name="John",
            relations={},
        )
    )
    world_graph.add_thing(
        Thing(
            id="alice",
            name="Alice",
            relations={},
        )
    )
    world_graph.add_thing(
        Thing(
            id="living_room",
            name="Living Room",
            relations={},
        )
    )

    _ = world_graph.create_relation("contains", "living_room", "john")
    _ = world_graph.create_relation("contains", "living_room", "alice")
    _ = world_graph.create_relation("likes", "john", "alice")

    test_query_exists(
        world_graph, "The living room contains John", "living_room.contains.john", True
    )
    test_query_exists(
        world_graph, "John is in the living room", "john.in.living_room", True
    )
    test_query_exists(
        world_graph, "The living room is in John", "living_room.in.john", False
    )
    test_query_exists(world_graph, "John likes Alice", "john.likes.alice", True)
    test_query_exists(world_graph, "Alice likes John", "alice.likes.john", False)
    test_query_exists(
        world_graph, "Alice is liked by John", "alice.liked_by.john", True
    )


if __name__ == "__main__":
    main()
