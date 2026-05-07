use js_sys::Function;
use praxsmth as core;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct World {
    inner: core::world::World,
    on_update: Option<Function>,
    on_dialog: Option<Function>,
}

#[wasm_bindgen]
impl World {
    pub fn new(types: String, world: String) -> World {
        console_error_panic_hook::set_once();
        let exposed_world = World {
            inner: core::world::World::from_strings(&types, &world).unwrap(),
            on_update: None,
            on_dialog: None,
        };

        exposed_world
    }

    #[wasm_bindgen(js_name = setOnUpdate)]
    pub fn set_on_update(&mut self, cb: Function) {
        self.on_update = Some(cb);
    }

    #[wasm_bindgen(js_name = setOnDialog)]
    pub fn set_on_dialog(&mut self, cb: Function) {
        self.on_dialog = Some(cb);
    }

    #[wasm_bindgen(js_name = getAgentNames)]
    pub fn get_agent_names(&self) -> JsValue {
        let agent_names: Vec<AgentInfo> = self
            .inner
            .agents
            .iter()
            .map(|(id, agent)| AgentInfo::new(id.clone(), agent.display_name.clone()))
            .collect();
        serde_wasm_bindgen::to_value(&agent_names).unwrap()
    }

    #[wasm_bindgen(js_name = getCurrentEmotion)]
    pub fn get_current_emotion(&self, agent: String) -> Option<String> {
        self.inner
            .get_current_emotion(&agent)
            .unwrap()
            .map(|rh_r| rh_r.1.type_name.clone())
    }

    #[wasm_bindgen(js_name = getAvailableActionNames)]
    pub fn get_available_action_names(&self, agent_name: String) -> Vec<String> {
        self.inner.get_available_action_names(&agent_name).unwrap()
    }

    #[wasm_bindgen(js_name = applyAction)]
    pub fn apply_action(&mut self, agent_name: String, action_index: u32) -> JsValue {
        let dialogs: Vec<Dialog> = self
            .inner
            .apply_action(&agent_name, action_index)
            .unwrap()
            .into_iter()
            .map(Dialog::from)
            .collect();
        for dialog in &dialogs {
            self.trigger_on_dialog(dialog);
        }
        self.trigger_on_update();
        serde_wasm_bindgen::to_value(&dialogs).unwrap()
    }
}

impl World {
    pub fn trigger_on_update(&self) {
        if let Some(cb) = &self.on_update {
            cb.call0(&JsValue::NULL).unwrap();
        }
    }

    fn trigger_on_dialog(&self, dialog: &Dialog) {
        if let Some(cb) = &self.on_dialog {
            let js_dialog = serde_wasm_bindgen::to_value(dialog).unwrap();
            cb.call1(&JsValue::NULL, &js_dialog).unwrap();
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Dialog {
    speaker: Option<String>,
    line: String,
}

impl From<core::world::simulation::Dialog> for Dialog {
    fn from(dialog: core::world::simulation::Dialog) -> Self {
        Dialog {
            speaker: dialog.speaker,
            line: dialog.line,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AgentInfo {
    id: String,
    name: String,
}

impl AgentInfo {
    pub fn new(id: String, name: String) -> Self {
        AgentInfo { id, name }
    }
}
