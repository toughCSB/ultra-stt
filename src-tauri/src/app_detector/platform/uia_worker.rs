use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{mpsc, OnceLock},
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{ChildStdin, ChildStdout},
    time::timeout_at,
};

use crate::local_process::{Limits, LocalProcess};

const HELPER_ARGUMENT: &str = "--ultra-stt-uia-helper";
const READY: &[u8; 4] = b"UIA3";
const QUERY_BUDGET: Duration = Duration::from_millis(150);
const STARTUP_BUDGET: Duration = Duration::from_secs(2);
const RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_ID_LENGTH: usize = 64;
const MAX_SELECTION_BYTES: usize = 65_536;

#[derive(Clone, Copy)]
enum Query {
    RuntimeId(u32),
    Selection { process_id: u32, window: u64 },
}

#[derive(Debug, PartialEq)]
enum Reply {
    RuntimeId(Vec<i32>),
    Selection(String),
}

struct Request {
    query: Query,
    deadline: Instant,
    reply: mpsc::Sender<Option<Reply>>,
}

struct Worker {
    sender: mpsc::SyncSender<Request>,
}

impl Worker {
    fn start(program: PathBuf) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Request>(1);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("target-focus-supervisor".into())
            .spawn(move || {
                let mut helper = runtime.block_on(Helper::start(&program)).ok();
                let mut retry_at = Instant::now() + RETRY_DELAY;
                while let Ok(request) = receiver.recv() {
                    if Instant::now() >= request.deadline {
                        let _ = request.reply.send(None);
                        continue;
                    }
                    if helper.is_none() {
                        let _ = request.reply.send(None);
                        if Instant::now() >= retry_at {
                            helper = runtime.block_on(Helper::start(&program)).ok();
                            retry_at = Instant::now() + RETRY_DELAY;
                        }
                        continue;
                    }
                    let result = runtime.block_on(
                        helper
                            .as_mut()
                            .unwrap()
                            .query(request.query, request.deadline),
                    );
                    let mut restart = false;
                    let value = match result {
                        Ok(value) if Instant::now() < request.deadline => value,
                        _ => {
                            drop(helper.take());
                            restart = true;
                            None
                        }
                    };
                    let _ = request.reply.send(value);
                    if restart {
                        std::thread::sleep(RETRY_DELAY);
                        helper = runtime.block_on(Helper::start(&program)).ok();
                        retry_at = Instant::now() + RETRY_DELAY;
                    }
                }
            })?;
        Ok(Self { sender })
    }

    fn query(&self, process_id: u32) -> Option<Vec<i32>> {
        match self.request(Query::RuntimeId(process_id))? {
            Reply::RuntimeId(value) => Some(value),
            _ => None,
        }
    }

    fn selection(&self, process_id: u32, window: u64) -> Option<String> {
        match self.request(Query::Selection { process_id, window })? {
            Reply::Selection(value) => Some(value),
            _ => None,
        }
    }

    fn request(&self, query: Query) -> Option<Reply> {
        let deadline = Instant::now() + QUERY_BUDGET;
        let (reply, receiver) = mpsc::channel();
        self.sender
            .try_send(Request {
                query,
                deadline,
                reply,
            })
            .ok()?;
        let value = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()?;
        (Instant::now() < deadline).then_some(value).flatten()
    }
}

fn worker() -> Option<&'static Worker> {
    static WORKER: OnceLock<Option<Worker>> = OnceLock::new();
    WORKER
        .get_or_init(|| Worker::start(std::env::current_exe().ok()?).ok())
        .as_ref()
}

pub(super) fn warm_up() {
    let _ = worker();
}

pub(super) fn focused_runtime_id(process_id: u32) -> Option<Vec<i32>> {
    worker()?.query(process_id)
}

pub(super) fn selected_text(process_id: u32, window: u64) -> Option<String> {
    worker()?.selection(process_id, window)
}

struct Helper {
    _owner: LocalProcess,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl Helper {
    async fn start(program: &Path) -> io::Result<Self> {
        let deadline = tokio::time::Instant::now() + STARTUP_BUDGET;
        let limits = Limits {
            stdin: 13,
            stdout: 4 + MAX_SELECTION_BYTES,
            stderr: 0,
            deadline: STARTUP_BUDGET,
        };
        let mut owner = LocalProcess::spawn(program, limits, |command| {
            command.arg(HELPER_ARGUMENT);
        })?;
        let stdin = owner
            .take_stdin()
            .ok_or_else(|| io::Error::other("UIA stdin unavailable"))?;
        let stdout = owner
            .take_stdout()
            .ok_or_else(|| io::Error::other("UIA stdout unavailable"))?;
        let mut helper = Self {
            _owner: owner,
            stdin,
            stdout,
        };
        let mut ready = [0; 4];
        timeout_at(deadline, helper.stdout.read_exact(&mut ready))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "UIA startup timed out"))??;
        if &ready != READY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "UIA protocol mismatch",
            ));
        }
        Ok(helper)
    }

    async fn query(&mut self, query: Query, deadline: Instant) -> io::Result<Option<Reply>> {
        timeout_at(deadline.into(), async {
            let (kind, process_id, window) = match query {
                Query::RuntimeId(process_id) => (0, process_id, 0),
                Query::Selection { process_id, window } => (1, process_id, window),
            };
            self.stdin.write_u8(kind).await?;
            self.stdin.write_all(&process_id.to_le_bytes()).await?;
            self.stdin.write_all(&window.to_le_bytes()).await?;
            self.stdin.flush().await?;
            if kind == 1 {
                let length = self.stdout.read_u32_le().await? as usize;
                // 확정된 빈 선택/조회 금지를 전송 실패와 구분한다.
                if length == u32::MAX as usize {
                    return Ok(Some(Reply::Selection(String::new())));
                }
                if length > MAX_SELECTION_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UIA selection too long",
                    ));
                }
                if length == 0 {
                    return Ok(None);
                }
                let mut bytes = vec![0; length];
                self.stdout.read_exact(&mut bytes).await?;
                let text = String::from_utf8(bytes)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                return Ok(Some(Reply::Selection(text)));
            }
            let length = self.stdout.read_u8().await? as usize;
            if length > MAX_ID_LENGTH {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "UIA runtime ID too long",
                ));
            }
            if length == 0 {
                return Ok(None);
            }
            let mut runtime_id = Vec::with_capacity(length);
            for _ in 0..length {
                runtime_id.push(self.stdout.read_i32_le().await?);
            }
            Ok(Some(Reply::RuntimeId(runtime_id)))
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "UIA query timed out"))?
    }
}

pub(crate) fn run_if_requested() -> bool {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() != Some(std::ffi::OsStr::new(HELPER_ARGUMENT)) {
        return false;
    }
    if arguments.next().is_none() {
        let _ = run_helper();
    }
    true
}

fn run_helper() -> io::Result<()> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(io::Error::other)?;
    let result = serve(
        io::stdin().lock(),
        io::stdout().lock(),
        |process_id| unsafe { super::windows::uia_runtime_id(process_id) },
        |process_id, window| unsafe { super::windows::uia_selected_text(process_id, window) },
    );
    unsafe {
        CoUninitialize();
    }
    result
}

fn serve(
    mut input: impl Read,
    mut output: impl Write,
    mut query: impl FnMut(u32) -> Option<Vec<i32>>,
    mut selection: impl FnMut(u32, u64) -> Option<String>,
) -> io::Result<()> {
    output.write_all(READY)?;
    output.flush()?;
    loop {
        let mut request = [0; 13];
        match input.read_exact(&mut request) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error),
        }
        let process_id = u32::from_le_bytes(request[1..5].try_into().unwrap());
        if request[0] == 1 {
            let window = u64::from_le_bytes(request[5..13].try_into().unwrap());
            let text = selection(process_id, window);
            if text.as_ref().is_some_and(|value| value.trim().is_empty()) {
                output.write_all(&u32::MAX.to_le_bytes())?;
                output.flush()?;
                continue;
            }
            let text = text.filter(|value| value.len() <= MAX_SELECTION_BYTES);
            let bytes = text.as_deref().unwrap_or_default().as_bytes();
            output.write_all(&(bytes.len() as u32).to_le_bytes())?;
            output.write_all(bytes)?;
            output.flush()?;
            continue;
        }
        if request[0] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown UIA query",
            ));
        }
        let runtime_id =
            query(process_id).filter(|value| !value.is_empty() && value.len() <= MAX_ID_LENGTH);
        let values = runtime_id.as_deref().unwrap_or_default();
        output.write_all(&[values.len() as u8])?;
        for value in values {
            output.write_all(&value.to_le_bytes())?;
        }
        output.flush()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        use std::os::windows::process::CommandExt;
        static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let executable = std::env::current_exe()
                    .unwrap()
                    .with_file_name(format!("uia-fixture-{}.exe", std::process::id()));
                let source =
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/uia_fixture.rs");
                let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
                let output = std::process::Command::new(compiler)
                    .arg("--edition=2021")
                    .arg(source)
                    .arg("-o")
                    .arg(&executable)
                    .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "UIA fixture build failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                executable
            })
            .clone()
    }

    fn await_healthy(worker: &Worker) -> Vec<i32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(value) = worker.query(42) {
                return value;
            }
            assert!(Instant::now() < deadline, "UIA worker did not recover");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn subsequent_query_recovers_after_provider_timeout() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::{
            Foundation::WAIT_OBJECT_0,
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        let worker = Worker::start(fixture()).unwrap();
        for _ in 0..5 {
            let original = await_healthy(&worker);
            let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, original[0] as u32) };
            assert!(!handle.is_null());
            let owned = unsafe { OwnedHandle::from_raw_handle(handle) };
            let started = Instant::now();
            assert_eq!(worker.query(1), None);
            assert!(started.elapsed() < Duration::from_millis(300));
            assert_eq!(
                unsafe { WaitForSingleObject(owned.as_raw_handle(), 2000) },
                WAIT_OBJECT_0
            );
            let recovered = await_healthy(&worker);
            assert_ne!(original[0], recovered[0]);
            assert_eq!(recovered[1], 42);
        }
    }

    #[test]
    fn crashed_malformed_and_partial_helpers_recover() {
        let worker = Worker::start(fixture()).unwrap();
        for bad_request in [2, 3, 4] {
            let original = await_healthy(&worker);
            assert_eq!(worker.query(bad_request), None);
            let recovered = await_healthy(&worker);
            assert_ne!(original[0], recovered[0]);
            assert_eq!(recovered[1], 42);
        }
    }

    #[test]
    fn healthy_helper_reused_and_none_is_not_a_crash() {
        let worker = Worker::start(fixture()).unwrap();
        let original = await_healthy(&worker);
        assert_eq!(worker.query(0), None);
        for _ in 0..20 {
            assert_eq!(worker.query(42), Some(original.clone()));
        }
    }

    #[test]
    fn expired_requests_never_reach_provider() {
        let worker = Worker::start(fixture()).unwrap();
        let original = await_healthy(&worker);
        let (reply, receiver) = mpsc::channel();
        worker
            .sender
            .send(Request {
                query: Query::RuntimeId(1),
                deadline: Instant::now(),
                reply,
            })
            .unwrap();
        assert_eq!(receiver.recv_timeout(Duration::from_secs(1)).unwrap(), None);
        assert_eq!(worker.query(42), Some(original));
    }

    #[test]
    fn concurrent_requests_remain_bounded_and_recover() {
        let worker = std::sync::Arc::new(Worker::start(fixture()).unwrap());
        await_healthy(&worker);
        let callers: Vec<_> = (0..16)
            .map(|_| {
                let worker = worker.clone();
                std::thread::spawn(move || {
                    let started = Instant::now();
                    assert_eq!(worker.query(1), None);
                    assert!(started.elapsed() < Duration::from_millis(300));
                })
            })
            .collect();
        for caller in callers {
            caller.join().unwrap();
        }
        assert_eq!(await_healthy(&worker)[1], 42);
    }

    #[tokio::test]
    async fn dropping_owner_kills_a_stuck_helper() {
        let mut helper = Helper::start(&fixture()).await.unwrap();
        helper.stdin.write_all(&[0]).await.unwrap();
        helper.stdin.write_all(&1u32.to_le_bytes()).await.unwrap();
        helper.stdin.write_all(&0u64.to_le_bytes()).await.unwrap();
        let Helper {
            _owner: owner,
            stdin,
            mut stdout,
        } = helper;
        drop(owner);
        drop(stdin);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), stdout.read_u8())
                .await
                .unwrap()
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn missing_executable_is_fail_closed_not_a_panic() {
        let worker = Worker::start(fixture().with_file_name("missing-uia-helper.exe")).unwrap();
        assert_eq!(worker.query(42), None);
    }

    #[test]
    fn selection_queries_preserve_unicode_and_recover_after_timeout() {
        let worker = Worker::start(fixture()).unwrap();
        let original = await_healthy(&worker);
        assert_eq!(
            worker.selection(42, 123),
            Some("선택된 글 / terminal selection".into())
        );
        assert_eq!(worker.selection(0, 123), None);
        assert_eq!(worker.selection(5, 123), Some(String::new()));
        assert_eq!(worker.query(42), Some(original.clone()));
        assert_eq!(worker.selection(1, 123), None);
        let recovered = await_healthy(&worker);
        assert_ne!(original[0], recovered[0]);
        assert_eq!(
            worker.selection(42, 123),
            Some("선택된 글 / terminal selection".into())
        );
    }

    #[test]
    fn oversized_and_invalid_utf8_selection_restarts_helper() {
        let worker = Worker::start(fixture()).unwrap();
        for invalid in [3, 4] {
            let original = await_healthy(&worker);
            assert_eq!(worker.selection(invalid, 123), None);
            let recovered = await_healthy(&worker);
            assert_ne!(original[0], recovered[0]);
        }
    }

    #[test]
    fn confirmed_empty_selection_is_distinct_from_unavailable_provider() {
        let input = [&[1][..], &42u32.to_le_bytes(), &123u64.to_le_bytes()].concat();
        let mut output = Vec::new();
        serve(
            input.as_slice(),
            &mut output,
            |_| panic!("selection must not request runtime IDs"),
            |_, _| Some(String::new()),
        )
        .unwrap();
        assert_eq!(output, [READY.as_slice(), &u32::MAX.to_le_bytes()].concat());
    }

    #[test]
    fn selection_protocol_rejects_oversize_and_never_requests_runtime_ids() {
        let mut output = Vec::new();
        let input: Vec<u8> = [42u32, 0, 1]
            .into_iter()
            .flat_map(|pid| [&[1][..], &pid.to_le_bytes(), &123u64.to_le_bytes()].concat())
            .collect();
        serve(
            input.as_slice(),
            &mut output,
            |_| panic!("selection request must not query runtime ids"),
            |pid, window| {
                assert_eq!(window, 123);
                match pid {
                    42 => Some("선택된 글".into()),
                    1 => Some("x".repeat(MAX_SELECTION_BYTES + 1)),
                    _ => None,
                }
            },
        )
        .unwrap();
        let text = "선택된 글".as_bytes();
        assert_eq!(
            output,
            [
                READY.as_slice(),
                &(text.len() as u32).to_le_bytes(),
                text,
                &0u32.to_le_bytes(),
                &0u32.to_le_bytes()
            ]
            .concat()
        );
    }

    #[test]
    fn protocol_is_bounded_and_carries_only_runtime_ids() {
        let mut output = Vec::new();
        let input: Vec<u8> = [42u32, 0, 1]
            .into_iter()
            .flat_map(|pid| [&[0][..], &pid.to_le_bytes(), &0u64.to_le_bytes()].concat())
            .collect();
        serve(
            input.as_slice(),
            &mut output,
            |pid| match pid {
                42 => Some(vec![10, -2]),
                1 => Some(vec![0; 65]),
                _ => None,
            },
            |_, _| panic!("runtime-id query must not capture selected text"),
        )
        .unwrap();
        let expected = [
            READY.as_slice(),
            &[2],
            &10i32.to_le_bytes(),
            &(-2i32).to_le_bytes(),
            &[0, 0],
        ]
        .concat();
        assert_eq!(output, expected);
    }
}
