#![cfg(windows)]

use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use opentypeless_lib::local_process::{Limits, LocalProcess};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn executable() -> PathBuf {
    std::env::var_os("ULTRA_STT_TEST_EXECUTABLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_ultra_stt")))
}

fn limits() -> Limits {
    Limits {
        stdin: 13,
        stdout: 65_540,
        stderr: 1024,
        deadline: Duration::from_secs(3),
    }
}

#[tokio::test]
async fn real_executable_dispatches_helper_before_app_startup() {
    let started = Instant::now();
    let mut child = LocalProcess::spawn(executable(), limits(), |command| {
        command.arg("--ultra-stt-uia-helper");
    })
    .unwrap();
    let mut stdout = child.take_stdout().unwrap();
    let mut ready = [0; 4];
    tokio::time::timeout(limits().deadline, stdout.read_exact(&mut ready))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&ready, b"UIA3");
    println!("real UIA helper startup: {:?}", started.elapsed());
    let mut stdin = child.take_stdin().unwrap();
    let mut samples = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        let length = tokio::time::timeout(limits().deadline, async {
            stdin.write_all(&[0]).await.unwrap();
            stdin.write_all(&u32::MAX.to_le_bytes()).await.unwrap();
            stdin.write_all(&0u64.to_le_bytes()).await.unwrap();
            stdin.flush().await.unwrap();
            stdout.read_u8().await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(length, 0);
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    println!(
        "real UIA unavailable-target query p50={:?}, p95={:?}, max={:?}",
        samples[9], samples[18], samples[19]
    );
    drop(stdin);
    assert!(child.wait(limits().deadline).await.unwrap().success());
    assert_eq!(
        stdout.read_u8().await.unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert!(child.stderr(limits().deadline).await.unwrap().is_empty());
}

#[tokio::test]
async fn extra_helper_arguments_cannot_launch_main_application() {
    let child = LocalProcess::spawn(executable(), limits(), |command| {
        command.args(["--ultra-stt-uia-helper", "unexpected"]);
    })
    .unwrap();
    let output = child.run(&[], limits()).await.unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn real_selection_query_with_no_target_returns_empty_without_app_startup() {
    let mut child = LocalProcess::spawn(executable(), limits(), |command| {
        command.arg("--ultra-stt-uia-helper");
    })
    .unwrap();
    let mut stdout = child.take_stdout().unwrap();
    let mut stdin = child.take_stdin().unwrap();
    let mut ready = [0; 4];
    tokio::time::timeout(limits().deadline, async {
        stdout.read_exact(&mut ready).await.unwrap();
        assert_eq!(&ready, b"UIA3");
        stdin.write_all(&[1]).await.unwrap();
        stdin.write_all(&u32::MAX.to_le_bytes()).await.unwrap();
        stdin.write_all(&0u64.to_le_bytes()).await.unwrap();
        stdin.flush().await.unwrap();
        assert_eq!(stdout.read_u32_le().await.unwrap(), 0);
    })
    .await
    .unwrap();
    drop(stdin);
    assert!(child.wait(limits().deadline).await.unwrap().success());
    assert!(child.stderr(limits().deadline).await.unwrap().is_empty());
}
