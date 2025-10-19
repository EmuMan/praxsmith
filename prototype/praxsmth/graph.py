from dataclasses import dataclass


@dataclass
class Thing:
    id: str
    name: str
    relations: dict[str, list["Relation"]]


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
    type: RelationType
    thing_1: Thing
    thing_2: Thing


class WorldGraph:
    things: dict[str, Thing]
    relation_types: dict[str, RelationType]
    relations: dict[str, list[Relation]]

    def __init__(self):
        self.things = {}
        self.relation_types = {}
        self.relations = {}

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

        relation = Relation(relation_type, thing_1, thing_2)

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

        if relation_type.forward_id not in thing_1.relations:
            thing_1.relations[relation_type.forward_id] = [relation]
        else:
            thing_1.relations[relation_type.forward_id].append(relation)
        if relation_type.backward_id not in thing_2.relations:
            thing_2.relations[relation_type.backward_id] = [relation]
        else:
            thing_2.relations[relation_type.backward_id].append(relation)

        return relation

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
        possible_relations = current_thing.relations[next_relation_id]
        for relation in possible_relations:
            other_thing = (
                relation.thing_1
                if relation.thing_2.id == current_thing.id
                else relation.thing_2
            )
            if other_thing.id == next_object_id:
                return self._query_exists_helper(other_thing, path_stack)
        return False
