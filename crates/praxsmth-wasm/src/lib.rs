use js_sys::Function;
use praxsmth as core;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// Convert any `anyhow::Error` (or other `std::error::Error`) into a JS `Error`
/// whose `.message` contains the full anyhow cause chain.
///
/// `anyhow::Error`'s `Debug` impl prints the chain like:
///
///     parsing world
///
///     Caused by:
///         0: agent disappeared between validation and emotion edge insertion
///         1: agent with name jacob already exists
///
/// That whole block ends up as the JS `Error.message`, which the frontend already
/// surfaces via `err.message` in its try/catch.
fn js_err<E: std::fmt::Debug>(err: E) -> JsError {
    JsError::new(&format!("{err:?}"))
}

#[wasm_bindgen]
pub struct World {
    inner: core::world::World,
    on_update: Option<Function>,
    on_dialog: Option<Function>,
}

#[wasm_bindgen]
impl World {
    #[wasm_bindgen(constructor)]
    pub fn new(types: String, world: String) -> Result<World, JsError> {
        console_error_panic_hook::set_once();
        let inner = core::world::World::from_strings(&types, &world).map_err(js_err)?;
        Ok(World {
            inner,
            on_update: None,
            on_dialog: None,
        })
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
    pub fn get_agent_names(&self) -> Result<JsValue, JsError> {
        let agent_names: Vec<AgentInfo> = self
            .inner
            .agents
            .iter()
            .map(|(id, agent)| AgentInfo::new(id.clone(), agent.name.clone()))
            .collect();
        serde_wasm_bindgen::to_value(&agent_names).map_err(js_err)
    }

    #[wasm_bindgen(js_name = getCurrentEmotion)]
    pub fn get_current_emotion(&self, agent: String) -> Result<Option<String>, JsError> {
        Ok(self
            .inner
            .get_current_emotion(&agent)
            .map_err(js_err)?
            .map(|(_, relation)| relation.type_name.clone()))
    }

    #[wasm_bindgen(js_name = getAvailableActionNames)]
    pub fn get_available_action_names(&self, agent_name: String) -> Result<Vec<String>, JsError> {
        self.inner
            .get_available_action_names(&agent_name)
            .map_err(js_err)
    }

    #[wasm_bindgen(js_name = applyAction)]
    pub fn apply_action(
        &mut self,
        agent_name: String,
        action_index: u32,
    ) -> Result<JsValue, JsError> {
        let dialogs: Vec<Dialog> = self
            .inner
            .apply_action(&agent_name, action_index)
            .map_err(js_err)?
            .into_iter()
            .map(Dialog::from)
            .collect();
        for dialog in &dialogs {
            self.trigger_on_dialog(dialog)?;
        }
        self.trigger_on_update()?;
        serde_wasm_bindgen::to_value(&dialogs).map_err(js_err)
    }
}

impl World {
    fn trigger_on_update(&self) -> Result<(), JsError> {
        if let Some(cb) = &self.on_update {
            let cb = cb.clone();
            let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
                let _ = cb.call0(&JsValue::NULL);
            });
            web_sys::window()
                .ok_or_else(|| JsError::new("no window"))?
                .set_timeout_with_callback_and_timeout_and_arguments_0(closure.unchecked_ref(), 0)
                .map_err(|e| JsError::new(&format!("setTimeout failed: {e:?}")))?;
        }
        Ok(())
    }

    fn trigger_on_dialog(&self, dialog: &Dialog) -> Result<(), JsError> {
        if let Some(cb) = &self.on_dialog {
            let cb = cb.clone();
            let js_dialog = serde_wasm_bindgen::to_value(dialog).map_err(js_err)?;
            let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
                let _ = cb.call1(&JsValue::NULL, &js_dialog);
            });
            web_sys::window()
                .ok_or_else(|| JsError::new("no window"))?
                .set_timeout_with_callback_and_timeout_and_arguments_0(closure.unchecked_ref(), 0)
                .map_err(|e| JsError::new(&format!("setTimeout failed: {e:?}")))?;
        }
        Ok(())
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
