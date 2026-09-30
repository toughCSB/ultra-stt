//! Owned, bounded local subprocesses. Dropping the owner terminates its process tree.
use std::{ffi::OsStr, io, process::{ExitStatus, Output, Stdio}, time::Duration};

use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
#[cfg(windows)]
use process_wrap::tokio::{CreationFlags, JobObject};
#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWriteExt}, process::{ChildStdin, ChildStdout}, task::JoinHandle, time::timeout};

/// Limits are bytes; the deadline applies to the complete operation, not each read.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub stdin: usize,
    pub stdout: usize,
    pub stderr: usize,
    pub deadline: Duration,
}

fn limit_error(stream: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{stream} exceeded byte limit"))
}

fn elapsed() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "local process deadline exceeded")
}

// Continue draining after overflow: otherwise a chatty child can block in write()
// and prevent the owner from reaching its wait/cleanup path.
async fn drain<R: AsyncRead + Unpin>(mut pipe: R, max: usize, name: &str) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut buf = [0; 8192];
    let mut overflow = false;
    loop {
        let n = pipe.read(&mut buf).await?;
        if n == 0 { break; }
        let keep = n.min(max.saturating_sub(data.len()));
        data.extend_from_slice(&buf[..keep]);
        overflow |= keep != n;
    }
    if overflow { Err(limit_error(name)) } else { Ok(data) }
}

/// The child wrapper (including its Job Object) is never unwrapped into a raw Child.
/// Stdout can be taken independently while this owner remains alive.
#[derive(Debug)]
pub struct LocalProcess {
    child: Box<dyn ChildWrapper>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<JoinHandle<io::Result<Vec<u8>>>>,
}

impl Drop for LocalProcess {
    fn drop(&mut self) {
        if let Some(task) = self.stderr.take() { task.abort(); }
        // child is dropped after this method, closing the kill-on-close Job on Windows.
    }
}

impl LocalProcess {
    /// Configure argv, environment and cwd on a Tokio Command, never a shell string.
    /// On Windows CREATE_NO_WINDOW is preserved while process-wrap temporarily suspends
    /// the child for atomic Job assignment. On Unix use an isolated process group.
    pub fn spawn(program: impl AsRef<OsStr>, limits: Limits, configure: impl FnOnce(&mut tokio::process::Command)) -> io::Result<Self> {
        let mut command = CommandWrap::with_new(program, |cmd| {
            configure(cmd);
            cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        });
        command.wrap(KillOnDrop);
        #[cfg(windows)] {
            command.wrap(CreationFlags(windows::Win32::System::Threading::CREATE_NO_WINDOW));
            command.wrap(JobObject);
        }
        #[cfg(unix)] command.wrap(ProcessGroup::leader());
        let mut child = command.spawn()?;
        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take().map(|pipe| tokio::spawn(drain(pipe, limits.stderr, "stderr")));
        Ok(Self { child, stdin, stdout, stderr })
    }

    pub fn id(&self) -> Option<u32> { self.child.id() }

    /// Writes at most `max` bytes and closes stdin, including on cancellation.
    pub async fn send_stdin(&mut self, payload: &[u8], max: usize, deadline: Duration) -> io::Result<()> {
        if payload.len() > max { return Err(limit_error("stdin")); }
        let mut stdin = self.stdin.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdin already closed"))?;
        timeout(deadline, stdin.write_all(payload)).await.map_err(|_| elapsed())??;
        drop(stdin);
        Ok(())
    }

    /// Reads exactly one bounded line, without buffering bytes past the ready event.
    pub async fn ready_line(&mut self, max: usize, deadline: Duration) -> io::Result<Vec<u8>> {
        let stdout = self.stdout.as_mut().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdout taken"))?;
        timeout(deadline, async {
            let mut line = Vec::new();
            loop {
                let byte = stdout.read_u8().await?;
                if line.len() == max { return Err(limit_error("ready line")); }
                line.push(byte);
                if byte == b'\n' { return Ok(line); }
            }
        }).await.map_err(|_| elapsed())?
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> { self.stdout.take() }

    /// Terminates the entire owned tree, then reaps it within the supplied deadline.
    pub async fn kill(&mut self, deadline: Duration) -> io::Result<()> {
        self.child.start_kill()?;
        timeout(deadline, self.child.wait()).await.map_err(|_| elapsed())??;
        Ok(())
    }

    pub async fn wait(&mut self, deadline: Duration) -> io::Result<ExitStatus> {
        timeout(deadline, self.child.wait()).await.map_err(|_| elapsed())?
    }

    /// Short CLI call: close stdin, concurrently drain both pipes and wait; dropping
    /// this future drops the owner and its Job. An overflow is reported after draining.
    pub async fn run(mut self, payload: &[u8], limits: Limits) -> io::Result<Output> {
        timeout(limits.deadline, async {
            self.send_stdin(payload, limits.stdin, limits.deadline).await?;
            let stdout = self.stdout.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stdout taken"))?;
            let stderr = self.stderr.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stderr taken"))?;
            let (status, stdout, stderr) = tokio::join!(self.child.wait(), drain(stdout, limits.stdout, "stdout"), stderr);
            let stderr = stderr.map_err(io::Error::other)??;
            Ok(Output { status: status?, stdout: stdout?, stderr })
        }).await.map_err(|_| elapsed())?
    }

    /// Consume a service owner while waiting. A timeout or cancelled future
    /// drops the Job, including descendants, even if stdout was taken out.
    pub async fn wait_owned(mut self, deadline: Duration) -> io::Result<ExitStatus> {
        self.wait(deadline).await
    }

    /// Collect bounded service diagnostics after the service exits.
    pub async fn stderr(&mut self, deadline: Duration) -> io::Result<Vec<u8>> {
        let task = self.stderr.take().ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "stderr already taken"))?;
        timeout(deadline, task).await.map_err(|_| elapsed())?.map_err(io::Error::other)?
    }
}
