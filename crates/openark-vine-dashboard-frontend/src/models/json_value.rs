use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use anyhow::{Result, anyhow, bail};
use itertools::Itertools;
use openark_vine_dashboard_api::item::{ItemField, ItemFieldKind, ItemMetadata};
use serde_json::{Map, Value};
use tracing::Level;
use yew::prelude::*;
use yewdux::Dispatch;

use crate::stores::client::{Alert, ClientStore};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JsonValue(RefCell<Map<String, Value>>);

pub enum JsonValueAction {
    Update { name: String, value: Value },
}

impl Reducible for JsonValue {
    type Action = JsonValueAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            JsonValueAction::Update { name, value } => {
                {
                    let mut root = self.0.borrow_mut();
                    root.insert(name, value);
                }
                self
            }
        }
    }
}

impl JsonValue {
    #[inline]
    pub fn new(value: Map<String, Value>) -> Self {
        Self(RefCell::new(value))
    }

    #[inline]
    pub fn borrow(&self) -> Ref<Map<String, Value>> {
        self.0.borrow()
    }

    /// Return `true` if all fields are valid.
    pub fn is_valid(&self, dispatch: &Dispatch<ClientStore>, metadata: &ItemMetadata) -> bool {
        let value = self.0.borrow();

        let error_message = metadata
            .template
            .fields
            .iter()
            .filter_map(|field| validate_field(field, &*value).err())
            .map(|error| error.to_string())
            .join("\n");

        if error_message.is_empty() {
            true
        } else {
            dispatch.reduce_mut(|client| {
                client.register_alert(Alert {
                    level: Level::ERROR,
                    message: error_message,
                })
            });
            false
        }
    }
}

pub fn to_form_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(*value as _),
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

pub fn to_form_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        value => Some(value.to_string()),
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(_) | Value::Number(_) => false,
        Value::String(s) => s.is_empty(),
        Value::Array(list) => list.is_empty(),
        Value::Object(map) => map.is_empty(),
    }
}

fn validate_field(field: &ItemField, value: &Map<String, Value>) -> Result<()> {
    let ItemField {
        name,
        kind,
        optional,
        title: _,
        description: _,
        default,
        placeholder: _,
        max_length,
        min_length,
        max_value,
        min_value,
    } = field;

    let value = match value.get(name) {
        Some(value) => value,
        None => default,
    };

    // Required field
    if !*optional && is_empty(value) {
        bail!("Required field: {name}");
    }

    match *kind {
        ItemFieldKind::Integer => {
            let value = to_form_i64(value).ok_or_else(|| anyhow!("Invalid number: {name}"))?;
            if let Some(max_value) = to_form_i64(max_value) {
                if value > max_value {
                    bail!("Value should be less than {max_value}: {name}");
                }
            }
            if let Some(min_value) = to_form_i64(min_value) {
                if value < min_value {
                    bail!("Value should be more than {min_value}: {name}");
                }
            }
            Ok(())
        }
        ItemFieldKind::String => {
            let value = to_form_string(value).ok_or_else(|| anyhow!("Invalid text: {name}"))?;
            if let Some(max_length) = *max_length {
                if value.len() > max_length {
                    bail!("Value should be less than {max_length} characters: {name}");
                }
            }
            {
                if value.len() < *min_length {
                    bail!("Value should be more than {min_length} characters: {name}");
                }
            }
            Ok(())
        }
    }
}
