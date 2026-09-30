use opentypeless_lib::stt::{SttConfig, SttProvider, ultra_local::UltraLocalProvider};

#[tokio::test]
async fn occupied_foreign_port_is_never_adopted_or_terminated() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    std::env::set_var("ULTRA_STT_ROOT", root);
    let foreign = std::net::TcpListener::bind("127.0.0.1:17845").unwrap();
    let mut provider = UltraLocalProvider::new();
    let result = tokio::time::timeout(std::time::Duration::from_secs(120),
        provider.connect(&SttConfig { language: Some("ko".into()), ..Default::default() })).await.unwrap();
    assert!(result.is_err(), "an occupied foreign port must not be adopted");
    assert!(foreign.local_addr().is_ok(), "native startup must not kill a foreign listener");
    drop(foreign);
    assert!(std::net::TcpListener::bind("127.0.0.1:17845").is_ok());
}
