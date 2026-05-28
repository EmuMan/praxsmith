use js_sys::Function;
use praxsmth::{self as core};
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
///         0: actor disappeared between validation and emotion edge insertion
///         1: actor with name jacob already exists
///
/// That whole block ends up as the JS `Error.message`, which the frontend already
/// surfaces via `err.message` in its try/catch.
fn js_err<E: std::fmt::Debug>(err: E) -> JsError {
    JsError::new(&format!("{err:?}"))
}

#[wasm_bindgen]
pub struct PraxsmthApi {
    inner: core::api::PraxsmthApi,
    on_update: Option<Function>,
    on_dialog: Option<Function>,
}

#[wasm_bindgen]
impl PraxsmthApi {
    #[wasm_bindgen(constructor)]
    pub fn new(types: String, world: String) -> Result<PraxsmthApi, JsError> {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Trace).ok();
        let inner = core::api::PraxsmthApi::from_strings(&types, &world).map_err(js_err)?;
        Ok(PraxsmthApi {
            inner,
            on_update: None,
            on_dialog: None,
        })
    }

    #[wasm_bindgen(js_name = processEffect)]
    pub fn process_effect(
        &mut self,
        actor_name: String,
        input: String,
    ) -> Result<Option<Dialog>, JsError> {
        let dialog = self
            .inner
            .process_effect(&actor_name, &input)
            .map_err(js_err)?
            .map(Dialog::from);
        if let Some(ref dialog) = dialog {
            self.trigger_on_dialog(dialog)?;
        }
        self.trigger_on_update()?;
        Ok(dialog)
    }

    #[wasm_bindgen(js_name = evaluateExpression)]
    pub fn evaluate_expression(&self, input: String) -> Result<PraxsmthConstant, JsError> {
        self.inner
            .evaluate_expression(&input)
            .map_err(js_err)
            .map(PraxsmthConstant::from)
    }

    #[wasm_bindgen(js_name = setOnUpdate)]
    pub fn set_on_update(&mut self, cb: Function) {
        self.on_update = Some(cb);
    }

    #[wasm_bindgen(js_name = setOnDialog)]
    pub fn set_on_dialog(&mut self, cb: Function) {
        self.on_dialog = Some(cb);
    }

    #[wasm_bindgen(js_name = getActorInfo)]
    pub fn get_actor_info(&self) -> Result<JsValue, JsError> {
        let actor_infos: Vec<ActorInfo> = self
            .inner
            .get_actor_info()
            .into_iter()
            .map(ActorInfo::from)
            .collect();
        serde_wasm_bindgen::to_value(&actor_infos).map_err(js_err)
    }

    #[wasm_bindgen(js_name = getRelationInfo)]
    pub fn get_relation_info(&self) -> Result<JsValue, JsError> {
        let relation_infos: Vec<RelationInfo> = self
            .inner
            .get_relation_info()
            .into_iter()
            .map(RelationInfo::from)
            .collect();
        serde_wasm_bindgen::to_value(&relation_infos).map_err(js_err)
    }

    #[wasm_bindgen(js_name = getCurrentEmotion)]
    pub fn get_current_emotion(&self, actor: String) -> Result<Option<String>, JsError> {
        Ok(self
            .inner
            .get_current_emotion(&actor)
            .map_err(js_err)?
            .map(|(_, relation)| relation.type_name.clone()))
    }

    #[wasm_bindgen(js_name = getAvailableActions)]
    pub fn get_available_actions(
        &mut self,
        actor_name: String,
        depth: usize,
    ) -> Result<JsValue, JsError> {
        let actions: Vec<AvailableAction> = self
            .inner
            .get_available_actions(&actor_name, depth)
            .map_err(js_err)?
            .into_iter()
            .map(AvailableAction::from)
            .collect();
        serde_wasm_bindgen::to_value(&actions).map_err(js_err)
    }

    #[wasm_bindgen(js_name = applyAction)]
    pub fn apply_action(
        &mut self,
        actor_name: String,
        action_index: u32,
    ) -> Result<JsValue, JsError> {
        let dialogs: Vec<Dialog> = self
            .inner
            .apply_action(&actor_name, action_index)
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

    #[wasm_bindgen(js_name = getDialogHistory)]
    pub fn get_dialog_history(&self) -> Result<JsValue, JsError> {
        let dialogs: Vec<Dialog> = self
            .inner
            .dialog_history
            .iter()
            .cloned()
            .map(Dialog::from)
            .collect();
        serde_wasm_bindgen::to_value(&dialogs).map_err(js_err)
    }
}

impl PraxsmthApi {
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
pub struct ActorInfo {
    id: String,
    name: String,
    active: bool,
}

impl From<core::api::ActorInfo> for ActorInfo {
    fn from(actor_info: core::api::ActorInfo) -> Self {
        ActorInfo {
            id: actor_info.id,
            name: actor_info.name,
            active: actor_info.active,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct RelationInfo {
    pub type_id: String,
    pub actors: Vec<String>,
    pub fields: Vec<(String, PraxsmthConstant)>,
    pub sentence: String,
}

impl From<core::api::RelationInfo> for RelationInfo {
    fn from(relation_info: core::api::RelationInfo) -> Self {
        RelationInfo {
            type_id: relation_info.type_id,
            actors: relation_info.actors,
            fields: relation_info
                .fields
                .into_iter()
                .map(|(k, v)| (k, PraxsmthConstant::from(v)))
                .collect(),
            sentence: relation_info.sentence,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AvailableAction {
    index: usize,
    display_name: String,
    score: f64,
}

impl From<core::api::AvailableAction> for AvailableAction {
    fn from(action: core::api::AvailableAction) -> Self {
        AvailableAction {
            index: action.index,
            display_name: action.display_name,
            score: action.goal_delta,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum PraxsmthConstant {
    Number(f64),
    Boolean(bool),
    Variant(String),
    String(String),
    ActorRef(String),
}

impl From<core::values::Constant> for PraxsmthConstant {
    fn from(constant: core::values::Constant) -> Self {
        match constant {
            core::values::Constant::Number(n) => PraxsmthConstant::Number(n),
            core::values::Constant::Boolean(b) => PraxsmthConstant::Boolean(b),
            core::values::Constant::Variant(v) => PraxsmthConstant::Variant(v),
            core::values::Constant::String(s) => PraxsmthConstant::String(s),
            core::values::Constant::ActorRef(r) => PraxsmthConstant::ActorRef(r),
        }
    }
}
