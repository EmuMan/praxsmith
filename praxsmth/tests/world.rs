use std::collections::HashMap;

use praxsmth::definitions::types::{PraxsmthType, PraxsmthTypeData};
use praxsmth::definitions::world::{AgentInfo, Declaration};
use praxsmth::types::TypeMapping;
use praxsmth::world::World;

fn setup_world() -> World {
    let mut type_mapping = TypeMapping::new();

    type_mapping
        .add_type(PraxsmthType {
            name: "chronically_sleep_deprived".into(),
            fields: HashMap::new(),
            data: PraxsmthTypeData::Emotion,
        })
        .unwrap();

    let mut world = World::new(type_mapping);
    world
        .add_agent(AgentInfo {
            name: "jacob".into(),
            subagents: HashMap::new(),
        })
        .unwrap();

    world
}

#[test]
fn test_trait() -> Result<(), String> {
    let mut world = setup_world();

    world.process_declaration(Declaration {
        sentence: vec![
            "jacob".into(),
            "is".into(),
            "chronically_sleep_deprived".into(),
        ],
        fields: HashMap::new(),
    })?;

    let jacob = world.get_agent("jacob").ok_or("could not find jacob")?;
    let new_edge_handle = jacob.edges.get(0).ok_or("jacob has no edges")?;
    world
        .get_relation(new_edge_handle.handle())
        .ok_or("could not find edge")?;

    Ok(())
}
