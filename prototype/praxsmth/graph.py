from dataclasses import dataclass
from typing import TypeVar, Callable


T = TypeVar("T")


@dataclass(frozen=True)
class SimpleHandle:
    index: int
    generation: int


@dataclass
class Thing:
    id: str
    name: str
    # first key is forward/backward id, second is thing
    relations: dict[str, dict[str, "Relation"]]


@dataclass
class RelationType:
    id: str

    forward_id: str
    forward_name: str
    forward_exclusive: bool

    backward_id: str
    backward_name: str
    backward_exclusive: bool

    parameters: list[str]
    reason: str | None = None


@dataclass
class Relation:
    parent_graph: "WorldGraph"
    handle: SimpleHandle
    type: RelationType
    thing_1: Thing
    thing_2: Thing

    def remove(self):
        assert self.parent_graph is not None
        assert self.parent_graph.remove_relation_from_handle(self.handle)


def remove_matching(items: list[T], condition: Callable[[T], bool]) -> int:
    """
    Remove all items from the list that match the given condition.

    Args:
        items: The list to remove items from (modified in-place)
        condition: A callable that returns True for items to remove

    Returns:
        The number of items removed

    Example:
        >>> numbers = [1, 2, 3, 4, 5, 6]
        >>> removed = remove_matching(numbers, lambda x: x % 2 == 0)
        >>> print(f"Removed {removed} items: {numbers}")
        Removed 3 items: [1, 3, 5]
    """
    original_length = len(items)
    items[:] = [item for item in items if not condition(item)]
    return original_length - len(items)


class WorldGraph:
    things: dict[str, Thing]
    relation_types: dict[str, RelationType]
    relations: dict[SimpleHandle, Relation]

    next_relation_index: int
    free_relation_handles: list[SimpleHandle]

    def __init__(self):
        self.things = {}
        self.relation_types = {}
        self.relations = {}
        self.next_relation_index = 0
        self.free_relation_handles = []

    def add_thing(self, thing: Thing):
        assert thing.id not in self.things
        self.things[thing.id] = thing

    def add_relation_type(self, relation_type: RelationType):
        assert relation_type.id not in self.relation_types
        self.relation_types[relation_type.id] = relation_type

    def create_relation(
        self, relation_type_id: str, thing_1_id: str, thing_2_id: str
    ) -> Relation | None:
        thing_1 = self.things[thing_1_id]
        thing_2 = self.things[thing_2_id]
        relation_type = self.relation_types[relation_type_id]

        # TODO: This should overwrite the previous relations, not ignore the new one
        if (
            relation_type.forward_exclusive
            and relation_type.forward_id in thing_1.relations
        ):
            return None
        if (
            relation_type.backward_exclusive
            and relation_type.backward_id in thing_2.relations
        ):
            return None

        # cannot have same relation between two objects
        if (
            relation_type.forward_id in thing_1.relations
            and thing_2.id in thing_1.relations[relation_type.forward_id]
        ):
            return None
        if (
            relation_type.backward_id in thing_2.relations
            and thing_1.id in thing_2.relations[relation_type.backward_id]
        ):
            return None

        # create the relation
        if self.free_relation_handles:
            relation_handle = self.free_relation_handles.pop()
        else:
            relation_handle = SimpleHandle(self.next_relation_index, 0)
            self.next_relation_index += 1
        relation = Relation(self, relation_handle, relation_type, thing_1, thing_2)
        self.relations[relation_handle] = relation

        # assign the relation to each objects
        if relation_type.forward_id not in thing_1.relations:
            thing_1.relations[relation_type.forward_id] = {thing_2_id: relation}
        else:
            thing_1.relations[relation_type.forward_id][thing_2_id] = relation
        if relation_type.backward_id not in thing_2.relations:
            thing_2.relations[relation_type.backward_id] = {thing_1_id: relation}
        else:
            thing_2.relations[relation_type.backward_id][thing_1_id] = relation

        return relation

    def get_relation(
        self, relation_type_id: str, thing_1_id: str, thing_2_id: str
    ) -> Relation | None:
        thing_1 = self.things[thing_1_id]
        relation_type = self.relation_types[relation_type_id]

        if (
            relation_type.forward_id in thing_1.relations
            and thing_2_id in thing_1.relations[relation_type.forward_id]
        ):
            return thing_1.relations[relation_type.forward_id][thing_2_id]

        return None

    def remove_relation_from_handle(self, relation_handle: SimpleHandle) -> bool:
        if relation_handle not in self.relations:
            return False

        relation = self.relations.pop(relation_handle)

        _ = relation.thing_1.relations[relation.type.forward_id].pop(
            relation.thing_2.id
        )
        _ = relation.thing_2.relations[relation.type.backward_id].pop(
            relation.thing_1.id
        )

        return True

    def query_exists(self, sentence: str) -> bool:
        components = sentence.split(".")
        current_thing_id, path_stack = components[0], components[1:]
        path_stack.reverse()
        return self._query_exists_helper(self.things[current_thing_id], path_stack)

    def _query_exists_helper(self, current_thing: Thing, path_stack: list[str]) -> bool:
        if not path_stack:
            return True

        next_relation_id = path_stack.pop()
        next_object_id = path_stack.pop()

        if next_relation_id not in current_thing.relations:
            return False

        if next_object_id in current_thing.relations[next_relation_id]:
            next_relation = current_thing.relations[next_relation_id][next_object_id]
            if next_relation.thing_1.id == next_object_id:
                return self._query_exists_helper(next_relation.thing_1, path_stack)
            else:
                return self._query_exists_helper(next_relation.thing_2, path_stack)

        return False

    def remove_relationship(self, sentence: str) -> bool:
        thing_a_id, relation_id, thing_b_id = sentence.split(".")

        thing_a = self.things[thing_a_id]
        thing_b = self.things[thing_b_id]

        if relation_id not in thing_a.relations:
            return False

        if thing_b.id not in thing_a.relations[relation_id]:
            return False

        thing_a.relations[relation_id][thing_b.id].remove()

        return True
