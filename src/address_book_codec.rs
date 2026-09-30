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

pub(crate) fn css_color(value: &str) -> Result<u32,&'static str> {
    let hex = value.strip_prefix('#').ok_or("invalid_tag_color")?;
    if !hex.bytes().all(|byte|byte.is_ascii_hexdigit()) { return Err("invalid_tag_color"); }
    let number = u32::from_str_radix(hex,16).map_err(|_|"invalid_tag_color")?;
    match hex.len() {
        3 => Ok(0xff000000 | ((number>>8)*17<<16) | (((number>>4)&15)*17<<8) | ((number&15)*17)),
        6 => Ok(0xff000000 | number),
        8 => Ok((number>>8) | (number<<24)), // CSS RRGGBBAA -> ARGB
        _ => Err("invalid_tag_color"),
    }
}

pub(crate) fn web_color(value: u32) -> String {
    if value>>24==255 { format!("#{:06x}",value&0xffffff) }
    else { format!("#{:06x}{:02x}",value&0xffffff,value>>24) }
}

pub(crate) fn normalize_colors(document: &mut Map<String,Value>) -> Result<(),&'static str> {
    let raw = match document.get("tag_colors") {
        Some(Value::String(text)) => serde_json::from_str(text).map_err(|_|"invalid_tag_colors")?,
        Some(value) => value.clone(),
        None => Value::Object(Map::new()),
    };
    let colors = raw.as_object().ok_or("invalid_tag_colors")?;
    let mut result = Map::new();
    for (name,value) in colors {
        let color = if let Some(value) = value.as_u64() { u32::try_from(value).map_err(|_|"invalid_tag_colors")? }
            else if let Some(value) = value.as_str() { css_color(value).map_err(|_|"invalid_tag_colors")? }
            else { return Err("invalid_tag_colors"); };
        result.insert(name.clone(),Value::from(color));
    }
    document.insert("tag_colors".to_owned(),Value::Object(result));
    Ok(())
}

pub(crate) fn legacy_colors(document: &mut Map<String,Value>) -> Result<(),&'static str> {
    if let Some(colors) = document.get("tag_colors") {
        let encoded = serde_json::to_string(colors).map_err(|_|"invalid_tag_colors")?;
        document.insert("tag_colors".to_owned(),Value::String(encoded));
    }
    Ok(())
}

pub(crate) fn rewrite_tag_refs(document: &mut Map<String,Value>, old: &str, new: Option<&str>) -> Result<(),&'static str> {
    if let Some(peers) = document.get_mut("peers").and_then(Value::as_array_mut) {
        for peer in peers {
            if let Some(raw) = peer.get_mut("tags") {
                let tags = raw.as_array_mut().ok_or("invalid_peer_tags")?;
                tags.retain_mut(|tag| {
                    let name = tag.as_str().or_else(||tag.get("name").and_then(Value::as_str));
                    if name.is_some_and(|name|name.eq_ignore_ascii_case(old)) {
                        if let Some(new) = new {
                            if tag.is_object() { tag["name"] = Value::String(new.to_owned()); }
                            else { *tag = Value::String(new.to_owned()); }
                        } else { return false; }
                    }
                    true
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_round_trips_argb_including_transparency_and_unsigned_maximum() {
        for color in [0,0x12345678,0xffabcdef,u32::MAX] { assert_eq!(css_color(&web_color(color)).unwrap(),color); }
        assert_eq!(css_color("#abc").unwrap(),0xffaabbcc);
        for invalid in ["#12","#ffggff","ffffff","#123456789"] { assert!(css_color(invalid).is_err()); }
    }
}

// Canonical integer DTO shared by Web conversion and new personal-book routes.
pub(crate) fn official_tag_values(document: &Map<String,Value>) -> Vec<Value> {
    let colors = document.get("tag_colors").and_then(Value::as_object);
    document.get("tags").and_then(Value::as_array).into_iter().flatten().filter_map(|tag| {
        let name = tag.as_str().or_else(||tag.get("name").and_then(Value::as_str))?;
        Some(serde_json::json!({"name":name,"color":colors.and_then(|colors|colors.get(name)).and_then(Value::as_u64).unwrap_or(0)}))
    }).collect()
}

#[cfg(test)]
mod official_tests {
    use super::*;
    #[test]
    fn new_tag_dto_uses_unsigned_argb_integers() {
        let mut document = serde_json::json!({"tags":["transparent","maximum"],"tag_colors":{"transparent":"#34567812","maximum":u32::MAX}}).as_object().unwrap().clone();
        normalize_colors(&mut document).unwrap();
        assert_eq!(official_tag_values(&document),vec![serde_json::json!({"name":"transparent","color":0x12345678u32}),serde_json::json!({"name":"maximum","color":u32::MAX})]);
    }
}
