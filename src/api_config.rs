use hbb_common::{bail, ResultType};
use hbbs::common;
use sodiumoxide::crypto::sign;

pub(crate) fn parse_bool_arg(name: &str, default: bool) -> ResultType<bool> {
    match common::get_arg_opt(name).as_deref() {
        None => Ok(default),
        Some("1" | "true" | "TRUE" | "yes" | "YES") => Ok(true),
        Some("0" | "false" | "FALSE" | "no" | "NO") => Ok(false),
        Some(value) => bail!("{name} must be a boolean, got {value}"),
    }
}

pub(crate) fn load_public_key() -> ResultType<String> {
    let value = match common::get_arg_opt("RUSTDESK_KEY") {
        Some(value) => value,
        None => {
            let path = common::get_arg_or("RUSTDESK_KEY_FILE", "id_ed25519.pub".to_owned());
            std::fs::read_to_string(path)?
        }
    };
    let decoded = base64::decode(value.trim())?;
    if decoded.len() != sign::PUBLICKEYBYTES {
        bail!("RUSTDESK_KEY must be a base64-encoded Ed25519 public key");
    }
    Ok(base64::encode(decoded))
}

