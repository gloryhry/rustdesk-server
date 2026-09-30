use hbb_common::{anyhow,bail,ResultType};
use std::{fs::{self,OpenOptions},io::Write,path::Path};

pub fn check_web_assets(root: &Path) -> ResultType<()> {
    let index = root.join("index.html");
    if fs::metadata(&index)?.len()>1024*1024 { bail!("Web index is too large"); }
    let html = fs::read_to_string(index)?;
    let references = regex::Regex::new(r#"(?:src|href)=["'](/assets/[a-zA-Z0-9_.-]+)["']"#)?;
    let mut scripts = 0;
    for capture in references.captures_iter(&html) {
        let path = capture.get(1).ok_or_else(||anyhow::anyhow!("invalid Web asset reference"))?.as_str();
        let metadata = fs::metadata(root.join(path.trim_start_matches('/')))?;
        if !metadata.is_file() || metadata.len()==0 { bail!("Web asset is missing or empty"); }
        if path.ends_with(".js") { scripts+=1; }
    }
    if scripts==0 { bail!("Web index must reference a built JavaScript asset"); }
    Ok(())
}

pub fn prepare_keypair(root: &Path) -> ResultType<()> {
    use sodiumoxide::crypto::sign;
    sodiumoxide::init().map_err(|_|anyhow::anyhow!("unable to initialize cryptography"))?;
    let private = root.join("id_ed25519"); let public = root.join("id_ed25519.pub");
    match (private.exists(),public.exists()) {
        (true,true) => {
            let private_bytes = base64::decode(fs::read_to_string(&private)?.trim()).map_err(|_|anyhow::anyhow!("invalid private key encoding"))?;
            let public_bytes = base64::decode(fs::read_to_string(&public)?.trim()).map_err(|_|anyhow::anyhow!("invalid public key encoding"))?;
            let secret = sign::SecretKey::from_slice(&private_bytes).ok_or_else(||anyhow::anyhow!("invalid private key length"))?;
            let public = sign::PublicKey::from_slice(&public_bytes).ok_or_else(||anyhow::anyhow!("invalid public key length"))?;
            let probe = b"rustdesk deployment key validation";
            if sign::verify(&sign::sign(probe,&secret),&public).map_or(true,|verified|verified!=probe) { bail!("RustDesk keypair does not match"); }
        },
        (false,false) => {
            let (pk,sk) = sign::gen_keypair();
            write_new_key(&private,base64::encode(sk).as_bytes(),true)?;
            // A single initializer runs under the container's flock. If public
            // persistence fails, keep the private key and fail closed; never rotate it.
            write_new_key(&public,base64::encode(pk).as_bytes(),false)?;
        },
        _ => bail!("incomplete RustDesk keypair; restore the matching key files"),
    }
    Ok(())
}
fn write_new_key(path: &Path, bytes: &[u8], private: bool) -> ResultType<()> {
    let mut options = OpenOptions::new(); options.create_new(true).write(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if private { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))] let _ = private;
    let mut file = options.open(path)?; file.write_all(bytes)?; file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialization_preserves_keys_and_rejects_partial_or_mismatched_pairs() {
        let root = std::env::temp_dir().join(format!("rustdesk-init-{}",uuid::Uuid::new_v4())); fs::create_dir(&root).unwrap();
        prepare_keypair(&root).unwrap(); let private = fs::read(root.join("id_ed25519")).unwrap(); let public = fs::read(root.join("id_ed25519.pub")).unwrap();
        prepare_keypair(&root).unwrap(); assert_eq!(fs::read(root.join("id_ed25519")).unwrap(),private);
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; assert_eq!(fs::metadata(root.join("id_ed25519")).unwrap().permissions().mode()&0o777,0o600); }
        let (other,_) = sodiumoxide::crypto::sign::gen_keypair(); fs::write(root.join("id_ed25519.pub"),base64::encode(other)).unwrap(); assert!(prepare_keypair(&root).is_err());
        assert_eq!(fs::read(root.join("id_ed25519")).unwrap(),private);
        fs::write(root.join("id_ed25519.pub"),public).unwrap(); fs::remove_file(root.join("id_ed25519.pub")).unwrap(); assert!(prepare_keypair(&root).is_err()); assert_eq!(fs::read(root.join("id_ed25519")).unwrap(),private); fs::remove_dir_all(root).unwrap();
    }
}
