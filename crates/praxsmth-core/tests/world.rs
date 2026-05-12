use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use praxsmth::definitions::PraxsmthValue;
use praxsmth::definitions::types::{
    Expression, PracticeAction, PracticeOutcome, PraxsmthType, PraxsmthTypeData,
};
use praxsmth::definitions::world::{AgentInfo, Declaration};
use praxsmth::types::TypeMapping;
use praxsmth::world::{Bindings, World};

fn setup_world() -> Result<World> {
    let mut type_mapping = TypeMapping::new();

    type_mapping.add_type(PraxsmthType {
        name: "chronically_sleep_deprived".into(),
        fields: HashMap::new(),
        data: PraxsmthTypeData::Emotion,
    })?;

    type_mapping.add_type(PraxsmthType {
        name: "wake".into(),
        fields: HashMap::new(),
        data: PraxsmthTypeData::Practice {
            params: vec!["waker".into(), "woken".into()],
            actions: vec![PracticeAction {
                for_actor: "waker".into(),
                name: "Wake".into(),
                conditions: vec![Expression::Value(PraxsmthValue::Variable(vec![
                    "woken".into(),
                    "feels".into(),
                    "chronically_sleep_deprived".into(),
                ]))],
                outcomes: vec![
                    PracticeOutcome::Say("AWOKEN".into()),
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

    world.add_agent(&AgentInfo {
        id: "jacob".into(),
        name: "Jacob".into(),
        active: true,
        subagents: HashMap::new(),
    })?;
    world.add_agent(&AgentInfo {
        id: "alaina".into(),
        name: "Alaina".into(),
        active: true,
        subagents: HashMap::new(),
    })?;

    Ok(world)
}

#[test]
fn test_trait() -> Result<()> {
    let mut world = setup_world()?;

    let sentence = vec![
        "jacob".into(),
        "feels".into(),
        "chronically_sleep_deprived".into(),
    ];

    world.process_declaration(
        &Declaration {
            sentence: sentence.clone(),
            fields: HashMap::new(),
        },
        &Bindings::default(),
    )?;

    let jacob = world.get_agent("jacob").context("could not find jacob")?;
    let new_edge_handle = jacob.edges.get(0).context("jacob has no edges")?;
    world
        .get_relation(new_edge_handle.handle())
        .context("could not find edge")?;

    if !world.check_condition(
        Expression::Value(PraxsmthValue::Variable(sentence)),
        &Bindings::default(),
    )? {
        bail!("jacob should be chronically sleep deprived");
    }

    Ok(())
}

#[test]
fn test_practice_ok() -> Result<()> {
    let mut world = setup_world()?;

    let emotion_sentence = vec![
        "jacob".into(),
        "feels".into(),
        "chronically_sleep_deprived".into(),
    ];

    world.process_declaration(
        &Declaration {
            sentence: emotion_sentence.clone(),
            fields: HashMap::new(),
        },
        &Bindings::default(),
    )?;

    world.process_declaration(
        &Declaration {
            sentence: vec![
                "practice".into(),
                "wake".into(),
                "alaina".into(),
                "jacob".into(),
            ],
            fields: HashMap::new(),
        },
        &Bindings::default(),
    )?;

    world.apply_action("alaina", 0)?;

    let jacob = world.get_agent("jacob").context("could not find jacob")?;
    if world.check_condition(
        Expression::Value(PraxsmthValue::Variable(emotion_sentence)),
        &Bindings::default(),
    )? {
        bail!("jacob should no longer be chronically sleep deprived");
    }
    if jacob.edges.len() != 1 {
        bail!("jacob should have one edge, has {}", jacob.edges.len());
    }

    Ok(())
}

#[test]
fn test_practice_condition_fail() -> Result<()> {
    let mut world = setup_world()?;

    world.process_declaration(
        &Declaration {
            sentence: vec![
                "practice".into(),
                "wake".into(),
                "alaina".into(),
                "jacob".into(),
            ],
            fields: HashMap::new(),
        },
        &Bindings::default(),
    )?;

    if !world.get_available_actions("alaina")?.is_empty() {
        bail!("alaina should have no available actions");
    }

    Ok(())
}
