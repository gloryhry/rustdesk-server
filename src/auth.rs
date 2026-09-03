use crate::database::{ApiUser, Database};
use hbb_common::tokio;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

const BCRYPT_COST: u32 = 12;
const MIN_PASSWORD_BYTES: usize = 8;
const MAX_PASSWORD_BYTES: usize = 72;

#[derive(Debug)]
pub enum AuthError {
    InvalidInput(&'static str),
    InvalidCredentials,
    UsernameUnavailable,
    Internal,
}

#[derive(Clone)]
pub struct AuthService {
    db: Database,
    secret: Arc<Vec<u8>>,
    token_ttl: Duration,
    issuer: String,
    audience: String,
    dummy_password_hash: Arc<String>,
    password_slots: Arc<tokio::sync::Semaphore>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub name: String,
    pub email: String,
    pub note: String,
    pub is_admin: bool,
    pub status: i64,
}

#[derive(Debug, Clone, Default)]
pub struct LoginDevice {
    pub id: String,
    pub uuid: String,
    pub name: String,
    pub os: String,
    pub device_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResult {
    #[serde(rename = "type")]
    pub token_type: &'static str,
    pub access_token: String,
    pub user: PublicUser,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisteredUser {
    pub id: String,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    jti: String,
    username: String,
    token_version: i64,
    iss: String,
    aud: String,
    iat: usize,
    exp: usize,
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub user: PublicUser,
    pub user_id: String,
    pub session_id: String,
}

impl AuthService {
    pub fn new(db: Database, secret: String, token_ttl: Duration) -> Result<Self, AuthError> {
        if secret.as_bytes().len() < 32 {
            return Err(AuthError::InvalidInput("API_JWT_SECRET must contain at least 32 bytes"));
        }
        let dummy_password_hash = bcrypt::hash("dummy-password", BCRYPT_COST)
            .map_err(|_| AuthError::Internal)?;
        Ok(Self {
            db,
            secret: Arc::new(secret.into_bytes()),
            token_ttl,
            issuer: "rustdesk-api".to_owned(),
            audience: "rustdesk-client".to_owned(),
            dummy_password_hash: Arc::new(dummy_password_hash),
            password_slots: Arc::new(tokio::sync::Semaphore::new(4)),
        })
    }

    pub fn db(&self) -> Database {
        self.db.clone()
    }

    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        make_first_user_admin: bool,
    ) -> Result<RegisteredUser, AuthError> {
        let username = normalize_username(username)?;
        let email = normalize_email(email)?;
        validate_password(password)?;
        let permit = self
            .password_slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AuthError::Internal)?;
        let password = password.to_owned();
        let password_hash = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            bcrypt::hash(password, BCRYPT_COST)
        })
        .await
        .map_err(|_| AuthError::Internal)?
        .map_err(|_| AuthError::Internal)?;
        let id = uuid::Uuid::new_v4().to_string();
        match self
            .db
            .create_api_user(
                &id,
                &username,
                &email,
                &password_hash,
                make_first_user_admin,
            )
            .await
        {
            Ok(()) => {
                let user = self
                    .db
                    .get_api_user_by_id(&id)
                    .await
                    .map_err(|_| AuthError::Internal)?
                    .ok_or(AuthError::Internal)?;
                Ok(RegisteredUser {
                    id,
                    username,
                    created_at: user.created_at,
                })
            }
            Err(err) if is_unique_violation(&err.to_string()) => {
                Err(AuthError::UsernameUnavailable)
            }
            Err(_) => Err(AuthError::Internal),
        }
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
        device: LoginDevice,
    ) -> Result<LoginResult, AuthError> {
        let username = normalize_username(username).map_err(|_| AuthError::InvalidCredentials)?;
        if password.as_bytes().len() > MAX_PASSWORD_BYTES {
            return Err(AuthError::InvalidCredentials);
        }
        let user = self
            .db
            .get_api_user_by_username(&username)
            .await
            .map_err(|_| AuthError::Internal)?;
        let (password_hash, user) = match user {
            Some(user) => (user.password_hash.clone(), Some(user)),
            None => ((*self.dummy_password_hash).clone(), None),
        };
        let permit = self
            .password_slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AuthError::Internal)?;
        let password = password.to_owned();
        let valid = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            bcrypt::verify(password, &password_hash)
        })
        .await
        .map_err(|_| AuthError::Internal)?
        .map_err(|_| AuthError::InvalidCredentials)?;
        let user = user.filter(|_| valid).filter(|user| user.status == 1);
        let user = user.ok_or(AuthError::InvalidCredentials)?;
        self.issue_login(user, device).await
    }

    pub async fn authorize(&self, token: &str) -> Result<Principal, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.leeway = 10;
        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validation,
        )
        .map_err(|_| AuthError::InvalidCredentials)?
        .claims;
        let user = self
            .db
            .get_api_user_by_id(&claims.sub)
            .await
            .map_err(|_| AuthError::Internal)?
            .filter(|user| user.status == 1 && user.token_version == claims.token_version)
            .ok_or(AuthError::InvalidCredentials)?;
        let now = crate::common::now() as i64;
        let session_active = self
            .db
            .is_api_session_active(&claims.jti, &claims.sub, now)
            .await
            .map_err(|_| AuthError::Internal)?;
        if !session_active {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(Principal {
            user: public_user(&user),
            user_id: user.id,
            session_id: claims.jti,
        })
    }

    pub async fn revoke_session(&self, user_id: &str, session_id: &str) -> Result<(), AuthError> {
        self.db
            .revoke_api_session(session_id, user_id, crate::common::now() as i64)
            .await
            .map_err(|_| AuthError::Internal)
    }

    pub async fn revoke_all(&self, user_id: &str) -> Result<(), AuthError> {
        self.db
            .increment_api_user_token_version(user_id)
            .await
            .map_err(|_| AuthError::Internal)
    }

    async fn issue_login(&self, user: ApiUser, device: LoginDevice) -> Result<LoginResult, AuthError> {
        let now = crate::common::now();
        let exp = now.saturating_add(self.token_ttl.as_secs());
        let session_id = uuid::Uuid::new_v4().to_string();
        self.db
            .create_api_session(
                &session_id,
                &user.id,
                &device.id,
                &device.uuid,
                &device.name,
                &device.os,
                &device.device_type,
                exp as i64,
            )
            .await
            .map_err(|_| AuthError::Internal)?;
        let claims = Claims {
            sub: user.id.clone(),
            jti: session_id,
            username: user.username.clone(),
            token_version: user.token_version,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now as usize,
            exp: exp as usize,
        };
        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|_| AuthError::Internal)?;
        Ok(LoginResult {
            token_type: "access_token",
            access_token,
            user: public_user(&user),
            expires_in: self.token_ttl.as_secs(),
        })
    }
}

pub fn normalize_username(username: &str) -> Result<String, AuthError> {
    let normalized = username.trim().to_ascii_lowercase();
    if !(3..=64).contains(&normalized.len())
        || !normalized
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'.' | b'_' | b'-'))
    {
        return Err(AuthError::InvalidInput("username must be 3-64 ASCII characters"));
    }
    Ok(normalized)
}

fn normalize_email(email: &str) -> Result<String, AuthError> {
    let normalized = email.trim().to_ascii_lowercase();
    if normalized.len() > 254
        || (!normalized.is_empty()
            && (!normalized.contains('@')
                || normalized.chars().any(char::is_whitespace)))
    {
        return Err(AuthError::InvalidInput("email is invalid"));
    }
    Ok(normalized)
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    let len = password.as_bytes().len();
    if !(MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES).contains(&len) {
        return Err(AuthError::InvalidInput("password length is invalid"));
    }
    Ok(())
}

fn public_user(user: &ApiUser) -> PublicUser {
    PublicUser {
        name: user.username.clone(),
        email: user.email.clone(),
        note: user.nickname.clone(),
        is_admin: user.is_admin != 0,
        status: user.status,
    }
}

fn is_unique_violation(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("unique") || message.contains("constraint")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_usernames() {
        assert_eq!(normalize_username(" Alice_1 ").unwrap(), "alice_1");
        assert!(normalize_username("ab").is_err());
        assert!(normalize_username("alice name").is_err());
    }

    #[test]
    fn enforces_bcrypt_input_limit() {
        assert!(validate_password(&"a".repeat(8)).is_ok());
        assert!(validate_password(&"a".repeat(73)).is_err());
    }

    #[hbb_common::tokio::test]
    async fn registers_logs_in_and_revokes_session() {
        let path = std::env::temp_dir().join(format!(
            "rustdesk-api-auth-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let path = path.to_string_lossy().to_string();
        let db = Database::new(&path).await.unwrap();
        let auth = AuthService::new(
            db,
            "01234567890123456789012345678901".to_owned(),
            Duration::from_secs(60),
        )
        .unwrap();
        let registered = auth
            .register("Alice", "alice@example.com", "password123", true)
            .await
            .unwrap();
        assert_eq!(registered.username, "alice");
        let login = auth
            .login("alice", "password123", LoginDevice::default())
            .await
            .unwrap();
        let principal = auth.authorize(&login.access_token).await.unwrap();
        assert_eq!(principal.user.name, "alice");
        auth
            .revoke_session(&principal.user_id, &principal.session_id)
            .await
            .unwrap();
        assert!(auth.authorize(&login.access_token).await.is_err());
        drop(auth);
        let _ = std::fs::remove_file(path);
    }
}
