//! Behavioral tests for applying effects to a world and observing the outcome
//! purely through the public `PraxsmthApi` surface: mutate with `apply_effect`,
//! then read the result back with `evaluate_expression`.

use praxsmth::api::PraxsmthApi;
use praxsmth::values::Constant;

const TYPES: &str = "\
trait rich { amount: 0..100 }
";

const WORLD: &str = "\
actor alice
actor bob
alice.is.rich { amount: 5 }
";

/// Fresh, fully-parsed world for a single test. `apply_effect` commits, so each
/// test gets its own world to stay isolated.
fn world() -> PraxsmthApi {
    PraxsmthApi::from_strings(TYPES, WORLD).expect("fixture should parse")
}

#[test]
fn update_changes_field() {
    let mut api = world();
    api.process_effect("alice", "update alice.is.rich.amount to 8")
        .unwrap();
    assert_eq!(
        api.evaluate_expression("alice.is.rich.amount").unwrap(),
        Constant::Number(8.0),
    );
}

#[test]
fn set_creates_relation() {
    let mut api = world();
    // bob has no `rich` trait in the fixture; `set` should create it.
    api.process_effect("bob", "set bob.is.rich { amount: 42 }")
        .unwrap();
    assert_eq!(
        api.evaluate_expression("bob.is.rich.amount").unwrap(),
        Constant::Number(42.0),
    );
}

#[test]
fn set_actor_active_works() {
    let mut api = world();
    api.process_effect("alice", "deactivate alice").unwrap();
    assert!(
        !api.get_actor_info()
            .iter()
            .find(|actor| actor.id == "alice")
            .expect("alice should be in actor info")
            .is_active
    );
}

#[test]
fn delete_removes_relation() {
    let mut api = world();
    api.process_effect("alice", "delete alice.is.rich").unwrap();
    assert_eq!(
        api.evaluate_expression("alice.is.rich").unwrap(),
        Constant::Boolean(false),
    );
}
