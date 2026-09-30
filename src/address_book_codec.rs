use serde::Deserialize;
use serde_json::{Map,Value};

fn relay(value: &Value) -> Result<bool,&'static str> {
    match value {
        Value::Bool(enabled) => Ok(*enabled),
        Value::String(value) if value=="true" => Ok(true),
        Value::String(value) if value=="false" => Ok(false),
        _ => Err("invalid_force_always_relay"),
    }
}

pub(crate) fn optional_relay<'de,D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<bool>,D::Error> {
    relay(&Value::deserialize(deserializer)?).map(Some).map_err(serde::de::Error::custom)
}

pub(crate) fn normalize_relays(document: &mut Map<String,Value>) -> Result<(),&'static str> {
    if let Some(peers) = document.get_mut("peers").and_then(Value::as_array_mut) {
        for peer in peers {
            if let Some(fields) = peer.as_object_mut() {
                let camel = fields.get("forceAlwaysRelay").map(relay).transpose()?;
                let snake = fields.get("force_always_relay").map(relay).transpose()?;
                if camel.is_some() && snake.is_some() && camel!=snake { return Err("conflicting_force_always_relay"); }
                if let Some(enabled) = camel.or(snake) {
                    fields.remove("force_always_relay");
                    fields.insert("forceAlwaysRelay".to_owned(),Value::Bool(enabled));
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn official_relays(document: &mut Map<String,Value>) {
    if let Some(peers) = document.get_mut("peers").and_then(Value::as_array_mut) {
        for peer in peers {
            if let Some(enabled) = peer.get("forceAlwaysRelay").and_then(Value::as_bool) {
                peer["forceAlwaysRelay"] = Value::String(enabled.to_string());
            }
        }
    }
}
