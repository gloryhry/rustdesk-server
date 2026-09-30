mod common;
use common::*;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};

#[cfg(target_os="linux")]
#[tokio::test]
async fn hbbs_version_checker_stores_its_client_configuration_only_in_the_test_directory() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    let directory = path.parent().unwrap();
    let _server = common::rendezvous::MockHbbs::start(directory,&path).await;
    let isolated = directory.join("client-config/rustdesk/RustDesk.toml");
    for _ in 0..100 {
        if isolated.is_file() { break; }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }
    assert!(isolated.is_file(),"hbbs must put its generated client configuration inside the disposable directory");
}
