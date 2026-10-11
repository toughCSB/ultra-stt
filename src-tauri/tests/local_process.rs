#![cfg(windows)]
use opentypeless_lib::local_process::{Limits, LocalProcess};
use std::{io, path::PathBuf, time::Duration};
use tokio::{io::AsyncReadExt, time::timeout};

const DEADLINE: Duration = Duration::from_secs(10);
fn fixture() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/process_fixture.exe")
}
fn limits() -> Limits {
    Limits {
        stdin: 1024,
        stdout: 1024,
        stderr: 1024,
        deadline: DEADLINE,
    }
}
fn spawn(mode: &str) -> io::Result<LocalProcess> {
    LocalProcess::spawn(fixture(), limits(), |cmd| {
        cmd.arg(mode);
    })
}
async fn tree() -> (LocalProcess, tokio::process::ChildStdout) {
    let mut owner = spawn("tree").unwrap();
    // The grandchild writes its own readiness signal to the inherited pipe.
    // Parent and grandchild lines may arrive in either order.
    let first = owner.ready_line(100, DEADLINE).await.unwrap();
    let second = owner.ready_line(100, DEADLINE).await.unwrap();
    assert!(first == b"parent-ready\n" || second == b"parent-ready\n");
    assert!(first == b"grandchild-ready\n" || second == b"grandchild-ready\n");
    let stdout = owner.take_stdout().unwrap();
    (owner, stdout)
}
async fn exited(mut stdout: tokio::process::ChildStdout) {
    let mut remaining = Vec::new();
    timeout(DEADLINE, stdout.read_to_end(&mut remaining))
        .await
        .unwrap()
        .unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn drop_closes_descendant_pipe() {
    let (owner, stdout) = tree().await;
    drop(owner);
    exited(stdout).await;
}

#[tokio::test]
async fn explicit_kill_closes_descendant_pipe() {
    let (mut owner, stdout) = tree().await;
    owner.kill(DEADLINE).await.unwrap();
    exited(stdout).await;
}

#[tokio::test]
async fn deadline_then_drop_closes_descendant_pipe() {
    let (owner, stdout) = tree().await;
    let err = owner
        .wait_owned(Duration::from_millis(50))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    exited(stdout).await;
}

#[tokio::test]
async fn cancelled_owned_wait_closes_descendant_pipe() {
    let (owner, stdout) = tree().await;
    let mut pending = Box::pin(owner.wait_owned(DEADLINE));
    assert!(matches!(
        futures_util::poll!(&mut pending),
        std::task::Poll::Pending
    ));
    drop(pending);
    exited(stdout).await;
}

#[tokio::test]
async fn cli_eof_and_bounded_output() {
    let output = spawn("echo")
        .unwrap()
        .run(b"hello", limits())
        .await
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"hello");
    assert_eq!(output.stderr, b"diagnostic");
    let err = spawn("flood")
        .unwrap()
        .run(b"", limits())
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}
