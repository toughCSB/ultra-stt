use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
};

fn main() -> io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("-m") => {
            let root = std::path::PathBuf::from(std::env::var_os("ULTRA_STT_ROOT").unwrap());
            let address = std::fs::read_to_string(root.join("foreign-address"))?;
            let mut bootstrap = String::new();
            io::stdin().read_line(&mut bootstrap)?;
            if !bootstrap.contains("session_secret") {
                return Err(io::Error::other("missing bootstrap"));
            }
            std::fs::write(root.join("helper-started"), "bind-attempt")?;
            let _listener = std::net::TcpListener::bind(address.trim())?;
            return Err(io::Error::other(
                "expected the fixture address to be occupied",
            ));
        }
        Some("grandchild") => {
            println!("grandchild-ready");
            io::stdout().flush()?;
            // Its parent keeps this pipe open. Only termination of the grandchild
            // closes the inherited stdout pipe observed by the test.
            io::stdin().read_to_end(&mut Vec::new())?;
        }
        Some("tree") => {
            let mut child = Command::new(std::env::current_exe()?)
                .arg("grandchild")
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;
            println!("parent-ready");
            io::stdout().flush()?;
            let _keep_grandchild_stdin_open = child.stdin.take();
            child.wait()?;
        }
        Some("echo") => {
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input)?;
            io::stdout().write_all(&input)?;
            io::stderr().write_all(b"diagnostic")?;
        }
        Some("flood") => {
            io::stdout().write_all(&vec![b'o'; 16384])?;
            io::stderr().write_all(&vec![b'e'; 16384])?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected tree, echo or flood",
            ))
        }
    }
    Ok(())
}
