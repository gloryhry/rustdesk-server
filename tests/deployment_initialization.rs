use std::{fs,path::Path,process::{Command,Output}};
use serde_json::Value;

fn initialize(root: &Path, settings: &[(&str,&str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rustdesk-api"));
    command.arg("--initialize").current_dir(root).env_clear()
        .env("API_ENABLED","1").env("API_PUBLIC_URL","https://api.example.com")
        .env("API_JWT_SECRET","isolated-deployment-signing-key-32-bytes")
        .env("API_OAUTH_CONFIG_KEY",base64::encode([7;32]))
        .env("API_BOOTSTRAP_ADMIN_USERNAME","isolated-admin")
        .env("API_BOOTSTRAP_ADMIN_PASSWORD","isolated-admin-password")
        .env("API_WEB_ROOT",root.join("web")).env("DB_URL",root.join("api.sqlite3"));
    for (key,value) in settings { command.env(key,value); } command.output().unwrap()
}
fn directory() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("rustdesk-deploy-{}",uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("web/assets")).unwrap();
    fs::write(root.join("web/index.html"),"<script type=\"module\" src=\"/assets/main.js\"></script>").unwrap();
    fs::write(root.join("web/assets/main.js"),"console.log('isolated-test')").unwrap(); root
}

#[tokio::test]
async fn initializer_migrates_and_bootstraps_once_without_listening_and_keeps_keys_on_restart() {
    let root = directory(); let output = initialize(&root,&[]);
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let private = fs::read(root.join("id_ed25519")).unwrap(); let public = fs::read(root.join("id_ed25519.pub")).unwrap();
    let pool = sqlx::SqlitePool::connect(root.join("api.sqlite3").to_str().unwrap()).await.unwrap();
    let id: String = sqlx::query_scalar("select id from api_user where username='isolated-admin'").fetch_one(&pool).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_schema_migration").fetch_one(&pool).await.unwrap(),3);
    assert!(initialize(&root,&[("API_BOOTSTRAP_ADMIN_USERNAME",""),("API_BOOTSTRAP_ADMIN_PASSWORD","")]).status.success());
    assert_eq!(fs::read(root.join("id_ed25519")).unwrap(),private); assert_eq!(fs::read(root.join("id_ed25519.pub")).unwrap(),public);
    assert_eq!(sqlx::query_scalar::<_,String>("select id from api_user where username='isolated-admin'").fetch_one(&pool).await.unwrap(),id);
    // Initializer output must contain no private key or passwords.
    let text = String::from_utf8_lossy(&output.stdout); assert!(!text.contains(std::str::from_utf8(&private).unwrap())); assert!(!text.contains("isolated-admin-password"));
    pool.close().await; fs::remove_dir_all(root).unwrap();
}

#[test]
fn initializer_fails_closed_for_missing_secrets_assets_admin_or_partial_keys() {
    for settings in [vec![("API_JWT_SECRET","")],vec![("API_OAUTH_CONFIG_KEY","")],vec![("API_BOOTSTRAP_ADMIN_USERNAME",""),("API_BOOTSTRAP_ADMIN_PASSWORD","")],vec![("API_OAUTH_CONFIG_KEY","bad-key")]] {
        let root = directory(); assert!(!initialize(&root,&settings).status.success()); fs::remove_dir_all(root).unwrap();
    }
    let root = directory(); fs::remove_file(root.join("web/assets/main.js")).unwrap(); assert!(!initialize(&root,&[]).status.success()); fs::remove_dir_all(root).unwrap();
    let root = directory(); fs::write(root.join("id_ed25519"),"preserve-this-partial-key").unwrap(); assert!(!initialize(&root,&[]).status.success()); assert_eq!(fs::read_to_string(root.join("id_ed25519")).unwrap(),"preserve-this-partial-key"); fs::remove_dir_all(root).unwrap();
}

#[test]
fn deployment_examples_do_not_supply_publicly_known_secrets_or_official_images() {
    for source in [include_str!("../docker-compose.yml"),include_str!("../kubernetes/example.yaml")] {
        assert!(!source.contains("image: rustdesk/rustdesk-server")); assert!(!source.contains("admin1234")); assert!(!source.contains("local-test-jwt-secret")); assert!(!source.contains(":latest"));
    }
    let samples: Value = serde_json::from_str(include_str!("fixtures/rustdesk-1.4.9-address-book.json")).unwrap(); assert_eq!(samples["version"],"1.4.9");
}
