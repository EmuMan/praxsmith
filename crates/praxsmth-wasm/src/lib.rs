use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use world_core as core;

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub bio: String,
    pub emotion: String,
}

impl From<core::Character> for Character {
    fn from(c: core::Character) -> Self {
        Self {
            id: c.id,
            name: c.name,
            bio: c.bio,
            emotion: c.emotion,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Message {
    pub id: u32,
    pub sender: String,
    pub text: String,
    pub system: bool,
}

impl From<core::Message> for Message {
    fn from(m: core::Message) -> Self {
        Self {
            id: m.id,
            sender: m.sender,
            text: m.text,
            system: m.system,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Action {
    pub id: String,
    pub label: String,
}

impl From<core::Action> for Action {
    fn from(a: core::Action) -> Self {
        Self {
            id: a.id,
            label: a.label,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct WorldState {
    pub characters: Vec<Character>,
    pub messages: Vec<Message>,
    pub actions: Vec<Action>,
    pub cycle: u32,
}

impl From<core::WorldState> for WorldState {
    fn from(s: core::WorldState) -> Self {
        Self {
            characters: s.characters.into_iter().map(Into::into).collect(),
            messages: s.messages.into_iter().map(Into::into).collect(),
            actions: s.actions.into_iter().map(Into::into).collect(),
            cycle: s.cycle,
        }
    }
}

#[wasm_bindgen]
pub struct World {
    inner: core::World,
}

#[wasm_bindgen]
impl World {
    #[wasm_bindgen(constructor)]
    pub fn new() -> World {
        console_error_panic_hook::set_once();
        World {
            inner: core::World::new(),
        }
    }

    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> WorldState {
        self.inner.state().clone().into()
    }

    #[wasm_bindgen(js_name = applyAction)]
    pub fn apply_action(&mut self, action_id: String) -> Message {
        self.inner.apply_action(&action_id).into()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
