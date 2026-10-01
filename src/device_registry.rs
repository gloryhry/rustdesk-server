//! Unsigned client telemetry, explicit administrator ownership, and hbbs observations.
use crate::database::Database;
use hbb_common::{log, tokio};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const REGISTRATION_TIMEOUT_MS: i64 = 30_000;
pub(crate) const REPORT_INTERVAL_MS: i64 = 5_000;
pub(crate) const MAX_REPORTS: i64 = 10_000;
pub(crate) const REPORT_MAX_BYTES: usize = 64 * 1024;

pub fn now_ms() -> i64 { chrono::Utc::now().timestamp_millis() }

pub fn key_fingerprint(pk: &[u8]) -> String {
    let digest = sodiumoxide::crypto::hash::sha256::hash(pk);
    let hex = digest.as_ref().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    format!("sha256:{hex}")
}

#[derive(Debug)]
pub(crate) enum RegistryError { Invalid(&'static str), NotFound, Conflict, RateLimited, Capacity, Storage }

pub(crate) fn report_identity(value: &Value) -> Result<(String, Vec<u8>), RegistryError> {
    let object = value.as_object().ok_or(RegistryError::Invalid("device_report_must_be_an_object"))?;
    let id = object.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 128)
        .ok_or(RegistryError::Invalid("invalid_device_id"))?;
    let uuid = object.get("uuid").and_then(Value::as_str).filter(|uuid| !uuid.is_empty() && uuid.len() <= 256)
        .ok_or(RegistryError::Invalid("invalid_device_uuid"))?;
    let uuid = [base64::STANDARD,base64::STANDARD_NO_PAD,base64::URL_SAFE,base64::URL_SAFE_NO_PAD]
        .into_iter().find_map(|config| base64::decode_config(uuid,config).ok())
        .filter(|uuid| !uuid.is_empty() && uuid.len() <= 128).ok_or(RegistryError::Invalid("invalid_device_uuid"))?;
    if serde_json::to_vec(value).map_err(|_| RegistryError::Storage)?.len() > REPORT_MAX_BYTES {
        return Err(RegistryError::Invalid("device_report_too_large"));
    }
    Ok((id.to_owned(),uuid))
}

#[derive(Deserialize)]
pub(crate) struct BindDeviceRequest { pub peer_id: String, pub user_id: String, pub pk_fingerprint: String }

#[derive(Serialize)]
pub(crate) struct RegisteredDeviceView {
    pub peer_id: String,
    pub uuid: String,
    pub pk_fingerprint: String,
    pub registered_at_ms: i64,
    pub online: bool,
    pub device_id: Option<String>,
    pub owner_id: Option<String>,
    pub verified: bool,
    pub untrusted_sysinfo: Option<Value>,
    pub untrusted_heartbeat: Option<Value>,
}

/// A successful registration is observed by hbbs, never by an unsigned API report.
#[derive(Clone)]
pub struct RegistrationObservation {
    pub guid: Vec<u8>,
    pub uuid: Vec<u8>,
    pub pk: Vec<u8>,
    pub registered_at_ms: i64,
}

#[derive(Clone)]
pub struct RegistrationWriter { sender: tokio::sync::mpsc::Sender<RegistrationObservation> }

impl RegistrationWriter {
    pub fn start(db: Database, capacity: usize) -> (Self, tokio::task::JoinHandle<()>) {
        let (sender,mut receiver) = tokio::sync::mpsc::channel(capacity.max(1));
        let task = tokio::spawn(async move {
            while let Some(first) = receiver.recv().await {
                let mut batch = vec![first];
                while batch.len() < 256 {
                    match receiver.try_recv() { Ok(value) => batch.push(value), Err(_) => break }
                }
                let mut saved = false;
                for _ in 0..3 {
                    if db.save_peer_registrations(&batch).await.is_ok() { saved = true; break; }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                if !saved { log::error!("Failed to persist hbbs registration batch; API online state remains conservative"); }
            }
        });
        (Self { sender },task)
    }

    /// Never waits for database I/O or a full queue. Dropped observations cannot make a peer online.
    pub fn observe(&self, observation: RegistrationObservation) -> bool {
        self.sender.try_send(observation).is_ok()
    }
}
