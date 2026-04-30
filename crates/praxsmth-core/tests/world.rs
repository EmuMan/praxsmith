use std::collections::HashMap;

use praxsmth::definitions::PraxsmthValue;
use praxsmth::definitions::types::{
    PracticeAction, PracticeCondition, PracticeOutcome, PraxsmthType, PraxsmthTypeData,
};
use praxsmth::definitions::world::{AgentInfo, Declaration};
use praxsmth::types::TypeMapping;
use praxsmth::world::World;

fn setup_world() -> Result<World, String> {
    let mut type_mapping = TypeMapping::new();

    type_mapping
        .add_type(PraxsmthType {
            name: "chronically_sleep_deprived".into(),
            fields: HashMap::new(),
            data: PraxsmthTypeData::Emotion,
        })
        .unwrap();

    type_mapping.add_type(PraxsmthType {
        name: "wake".into(),
        fields: HashMap::new(),
        data: PraxsmthTypeData::Practice {
            params: vec!["waker".into(), "woken".into()],
            display: Some("Wake".into()),
            actions: vec![PracticeAction {
                for_actor: "waker".into(),
                name: "Wake".into(),
                conditions: vec![PracticeCondition::Value(PraxsmthValue::Variable(vec![
                    "woken".into(),
                    "feels".into(),
                    "chronically_sleep_deprived".into(),
                ]))],
                outcomes: vec![
                    PracticeOutcome::Print("AWOKEN".into()),
                    PracticeOutcome::Delete(vec![
                        "woken".into(),
                        "feels".into(),
                        "chronically_sleep_deprived".into(),
                    ]),
                ],
            }],
        },
    })?;

    let mut world = World::new(type_mapping);

    world.add_agent(AgentInfo {
        name: "jacob".into(),
        subagents: HashMap::new(),
    })?;
    world.add_agent(AgentInfo {
        name: "alaina".into(),
        subagents: HashMap::new(),
    })?;

    Ok(world)
}

#[test]
fn test_trait() -> Result<(), String> {
    let mut world = setup_world()?;

    let sentence = vec![
        "jacob".into(),
        "feels".into(),
        "chronically_sleep_deprived".into(),
    ];

    world.process_declaration(Declaration {
        sentence: sentence.clone(),
        fields: HashMap::new(),
    })?;

    let jacob = world.get_agent("jacob").ok_or("could not find jacob")?;
    let new_edge_handle = jacob.edges.get(0).ok_or("jacob has no edges")?;
    world
        .get_relation(new_edge_handle.handle())
        .ok_or("could not find edge")?;

    if !world.check_condition(
        PracticeCondition::Value(PraxsmthValue::Variable(sentence)),
        &HashMap::new(),
    )? {
        return Err("jacob should be chronically sleep deprived".into());
    }

    Ok(())
}

#[test]
fn test_practice_ok() -> Result<(), String> {
    let mut world = setup_world()?;

    let emotion_sentence = vec![
        "jacob".into(),
        "feels".into(),
        "chronically_sleep_deprived".into(),
    ];

    world.process_declaration(Declaration {
        sentence: emotion_sentence.clone(),
        fields: HashMap::new(),
    })?;

    world.process_declaration(Declaration {
        sentence: vec![
            "practice".into(),
            "wake".into(),
            "alaina".into(),
            "jacob".into(),
        ],
        fields: HashMap::new(),
    })?;

    world.apply_action("alaina", 0)?;

    let jacob = world.get_agent("jacob").ok_or("could not find jacob")?;
    if world.check_condition(
        PracticeCondition::Value(PraxsmthValue::Variable(emotion_sentence)),
        &HashMap::new(),
    )? {
        return Err("jacob should no longer be chronically sleep deprived".into());
    }
    if jacob.edges.len() != 1 {
        return Err("jacob should have one edge".into());
    }

    Ok(())
}

#[test]
fn test_practice_condition_fail() -> Result<(), String> {
    let mut world = setup_world()?;

    world.process_declaration(Declaration {
        sentence: vec![
            "practice".into(),
            "wake".into(),
            "alaina".into(),
            "jacob".into(),
        ],
        fields: HashMap::new(),
    })?;

    if !world.get_available_actions("alaina")?.is_empty() {
        return Err("alaina should have no available actions".into());
    }

    Ok(())
}
