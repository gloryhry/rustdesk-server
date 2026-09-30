use crate::oauth::{ExternalIdentity, OAuthDevice};
use hbb_common::tokio::sync::Mutex;
use std::{collections::HashMap, sync::Arc};

pub(crate) const FLOW_TTL: u64 = 300;
const MAX_FLOWS: usize = 10_000;

enum Status {
    Waiting,
    Starting,
    Authorizing(String),
    Ready(ExternalIdentity),
    Failed(&'static str),
}

struct Flow {
    provider: String,
    device: OAuthDevice,
    launch: String,
    expires_at: u64,
    status: Status,
}

#[derive(Clone)]
pub(crate) struct NativeOAuthStore {
    flows: Arc<Mutex<HashMap<String, Flow>>>,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

impl NativeOAuthStore {
    pub fn new(clock: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        Self { flows: Arc::new(Mutex::new(HashMap::new())), clock }
    }

    pub async fn issue(&self, provider: String, device: OAuthDevice) -> Result<(String, String), &'static str> {
        if device.id.is_empty() || device.uuid.is_empty() || device.id.len() > 128
            || device.uuid.len() > 128 || device.name.len() > 256 || device.os.len() > 128
            || device.device_type.len() > 64
        {
            return Err("invalid_native_device");
        }
        let now = (self.clock)();
        let mut flows = self.flows.lock().await;
        flows.retain(|_, flow| flow.expires_at > now);
        if flows.len() >= MAX_FLOWS { return Err("too_many_native_authorizations"); }
        let code = capability();
        let launch = capability();
        flows.insert(code.clone(), Flow { provider, device, launch: launch.clone(),
            expires_at: now.saturating_add(FLOW_TTL), status: Status::Waiting });
        Ok((code, launch))
    }

    /// The launch capability only starts authorization; it cannot poll or retrieve credentials.
    pub async fn launch(&self, launch: &str) -> Result<(String, String, OAuthDevice), &'static str> {
        let mut flows = self.flows.lock().await;
        let (code, flow) = flows.iter_mut().find(|(_, flow)| flow.launch == launch)
            .ok_or("invalid_native_launch")?;
        if flow.expires_at <= (self.clock)() || !matches!(flow.status, Status::Waiting) {
            return Err("invalid_native_launch");
        }
        flow.status = Status::Starting;
        Ok((code.clone(), flow.provider.clone(), flow.device.clone()))
    }

    pub async fn bind(&self, code: &str, state: String) -> Result<(), &'static str> {
        let mut flows = self.flows.lock().await;
        let flow = flows.get_mut(code).ok_or("invalid_native_authorization")?;
        if flow.expires_at <= (self.clock)() || !matches!(flow.status, Status::Starting) {
            return Err("invalid_native_authorization");
        }
        flow.status = Status::Authorizing(state);
        Ok(())
    }

    pub async fn fail_start(&self, code: &str) {
        if let Some(flow) = self.flows.lock().await.get_mut(code) {
            if matches!(flow.status, Status::Starting) { flow.status = Status::Failed("oauth_provider_failed"); }
        }
    }

    pub async fn finish(&self, state: &str, result: Result<ExternalIdentity, &'static str>) -> Result<(), &'static str> {
        let mut flows = self.flows.lock().await;
        let flow = flows.values_mut().find(|flow|
            matches!(&flow.status, Status::Authorizing(value) if value == state))
            .ok_or("invalid_native_authorization")?;
        if flow.expires_at <= (self.clock)() { return Err("native_authorization_expired"); }
        flow.status = match result { Ok(identity) => Status::Ready(identity), Err(error) => Status::Failed(error) };
        Ok(())
    }

    pub async fn is_authorizing(&self, state: &str) -> bool {
        self.flows.lock().await.values().any(|flow|
            matches!(&flow.status, Status::Authorizing(value) if value == state))
    }

    pub async fn poll(&self, code: &str, id: &str, uuid: &str) -> Result<ExternalIdentity, &'static str> {
        let mut flows = self.flows.lock().await;
        let flow = flows.get(code).ok_or("invalid_native_authorization")?;
        if flow.device.id != id || flow.device.uuid != uuid { return Err("invalid_native_authorization"); }
        if flow.expires_at <= (self.clock)() {
            flows.remove(code);
            return Err("native_authorization_expired");
        }
        if matches!(flow.status, Status::Waiting | Status::Starting | Status::Authorizing(_)) {
            return Err("No authed oidc is found");
        }
        let flow = flows.remove(code).ok_or("invalid_native_authorization")?;
        match flow.status {
            Status::Ready(identity) => Ok(identity),
            Status::Failed(error) => Err(error),
            _ => Err("invalid_native_authorization"),
        }
    }
}

fn capability() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}
