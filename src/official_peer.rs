//! A native-client DTO independent of management IDs and database serialization.
use serde::Serialize;
use serde_json::{Map,Value};

#[derive(sqlx::FromRow)]
pub(crate) struct PeerRecord {
    pub peer_id: String,
    pub user_id: String,
    pub user_name: String,
    pub name: String,
    pub os: String,
    pub legacy_info: String,
    pub reported_info: Option<String>,
    pub status: i64,
    pub registered_at_ms: i64,
    pub online: bool,
    pub device_group_name: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct OfficialPeer {
    pub id: String,
    pub info: Map<String,Value>,
    pub status: i64,
    pub online: bool,
    pub registered_at_ms: i64,
    pub user: String,
    pub user_name: String,
    pub device_group_name: Option<String>,
    pub note: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub info_error: Option<&'static str>,
}

impl From<PeerRecord> for OfficialPeer {
    fn from(record: PeerRecord) -> Self {
        let mut info_error = None;
        let mut info = if record.legacy_info.is_empty() { Map::new() } else {
            match serde_json::from_str::<Value>(&record.legacy_info) {
                Ok(Value::Object(info)) => info,
                _ => { info_error = Some("invalid_legacy_device_info"); Map::new() }
            }
        };
        if let Some(report) = record.reported_info {
            match serde_json::from_str::<Value>(&report) {
                Ok(Value::Object(report)) => info.extend(report),
                _ => info_error = Some("invalid_report_device_info"),
            }
        }
        let username = info.get("username").and_then(Value::as_str).unwrap_or_default().to_owned();
        let os = info.get("os").and_then(Value::as_str).unwrap_or(&record.os).to_owned();
        let name = info.get("device_name").and_then(Value::as_str)
            .or_else(||info.get("hostname").and_then(Value::as_str)).unwrap_or(&record.name).to_owned();
        info.insert("username".to_owned(),Value::String(username));
        info.insert("os".to_owned(),Value::String(os));
        info.insert("device_name".to_owned(),Value::String(name));
        let note = info.get("note").and_then(Value::as_str).unwrap_or_default().to_owned();
        Self { id:record.peer_id,info,status:record.status,online:record.online,registered_at_ms:record.registered_at_ms,
            user:record.user_id,user_name:record.user_name,device_group_name:record.device_group_name,note,info_error }
    }
}
