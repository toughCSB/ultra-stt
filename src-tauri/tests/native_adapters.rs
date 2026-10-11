use opentypeless_lib::stt::{ultra_local::UltraLocalProvider, SttConfig, SttProvider};

#[tokio::test]
async fn occupied_foreign_port_is_never_adopted_or_terminated() {
    let root = std::env::temp_dir().join(format!("ultra-stt-adapter-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join(".venv/Scripts")).unwrap();
    std::fs::create_dir_all(root.join("opentypeless")).unwrap();
    let fixture = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!(
            "examples/process_fixture{}",
            std::env::consts::EXE_SUFFIX
        ));
    std::fs::copy(fixture, root.join(".venv/Scripts/python.exe")).unwrap();
    let previous_root = std::env::var_os("ULTRA_STT_ROOT");
    std::env::set_var("ULTRA_STT_ROOT", &root);
    let foreign = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = foreign.local_addr().unwrap();
    std::fs::write(root.join("foreign-address"), address.to_string()).unwrap();
    let mut provider = UltraLocalProvider::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        provider.connect(&SttConfig {
            language: Some("ko".into()),
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    assert!(
        result.is_err(),
        "an occupied foreign port must not be adopted"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("helper-started")).unwrap(),
        "bind-attempt"
    );
    assert!(
        foreign.local_addr().is_ok(),
        "native startup must not kill a foreign listener"
    );
    drop(foreign);
    assert!(std::net::TcpListener::bind(address).is_ok());
    if let Some(previous) = previous_root {
        std::env::set_var("ULTRA_STT_ROOT", previous);
    } else {
        std::env::remove_var("ULTRA_STT_ROOT");
    }
    assert!(root.starts_with(std::env::temp_dir()));
    std::fs::remove_dir_all(root).unwrap();
}
