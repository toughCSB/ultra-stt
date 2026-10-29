use super::{codex, ChunkCallback, LlmConfig, LlmProvider, PolishRequest, PolishResponse};
use crate::{
    error::AppError,
    local_process::{Limits, LocalProcess},
};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_PROMPT: usize = 64 * 1024;
const MAX_FINAL: usize = 64 * 1024;
const DEADLINE: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CliKind {
    Claude,
    Grok,
}

impl CliKind {
    fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Grok => "Grok CLI",
        }
    }

    fn model(self, model: &str) -> Result<(), AppError> {
        let allowed = match self {
            Self::Claude => matches!(model, "claude-haiku-5-5" | "claude-sonnet-5-5"),
            Self::Grok => model == "grok-4.7",
        };
        if allowed {
            Ok(())
        } else {
            Err(AppError::Config(format!(
                "Unsupported {} model",
                self.name()
            )))
        }
    }
}

fn limits(deadline: Duration) -> Limits {
    Limits {
        stdin: MAX_PROMPT,
        stdout: 4 * 1024 * 1024,
        stderr: 16 * 1024,
        deadline,
    }
}

fn neutral_cwd() -> Result<PathBuf, AppError> {
    let path = std::env::temp_dir().join("opentypeless-cli");
    std::fs::create_dir_all(&path)
        .map_err(|_| AppError::Config("Cannot prepare a neutral CLI working directory".into()))?;
    Ok(path)
}

#[cfg(windows)]
fn windows_claude_candidates(
    app_data: Option<PathBuf>,
    user_profile: Option<PathBuf>,
) -> Vec<PathBuf> {
    app_data
        .into_iter()
        .map(|path| path.join("npm/node_modules/@anthropic-ai/claude-code/bin/claude.exe"))
        .chain(
            user_profile
                .into_iter()
                .map(|path| path.join(".local/bin/claude.exe")),
        )
        .collect()
}

#[cfg(windows)]
fn claude_executable() -> Result<PathBuf, AppError> {
    use std::io::Read;

    windows_claude_candidates(
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("USERPROFILE").map(PathBuf::from),
    )
    .into_iter()
    .find(|path| {
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let mut magic = [0; 2];
        file.read_exact(&mut magic).is_ok() && magic == *b"MZ"
    })
    .ok_or_else(|| AppError::Config("Native Claude Code executable is unavailable".into()))
}

#[cfg(target_os = "macos")]
fn claude_executable() -> Result<PathBuf, AppError> {
    use std::io::Read;

    let home = std::env::var_os("HOME").ok_or_else(|| {
        AppError::Config("Cannot locate current-user Claude Code installation".into())
    })?;
    let home = PathBuf::from(home);
    [
        home.join(".local/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
    ]
    .into_iter()
    .find(|path| {
        let Ok(mut file) = std::fs::File::open(path) else {
            return false;
        };
        let mut magic = [0; 4];
        file.read_exact(&mut magic).is_ok()
            && matches!(
                magic,
                [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xbe, 0xba, 0xfe, 0xca]
            )
    })
    .ok_or_else(|| AppError::Config("Native Claude Code executable is unavailable".into()))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn claude_executable() -> Result<PathBuf, AppError> {
    Err(AppError::Config(
        "Claude Code CLI is unsupported on this desktop".into(),
    ))
}

fn scrub_api_credentials(command: &mut tokio::process::Command) {
    for name in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
        "XAI_API_KEY",
        "GROK_API_KEY",
        "OPENAI_API_KEY",
    ] {
        command.env_remove(name);
    }
}

fn claude_args(model: &str, effort: codex::ReasoningEffort) -> Vec<&str> {
    vec![
        "-p",
        "--safe-mode",
        "--restricted",
        "--strict-mcp-config",
        "--tools",
        "",
        "--disallowedTools",
        "mcp__*",
        "--disable-slash-commands",
        "--no-chrome",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--no-session-persistence",
        "--max-turns",
        "1",
        "--model",
        model,
        "--effort",
        effort.api_value(),
        "--output-format",
        "stream-json",
        "--verbose",
    ]
}

fn spawn_claude(args: &[&str], bound: Limits, cwd: &PathBuf) -> Result<LocalProcess, AppError> {
    let executable = claude_executable()?;
    LocalProcess::spawn(&executable, bound, |command| {
        command.current_dir(cwd).args(args);
        scrub_api_credentials(command);
    })
    .map_err(|_| AppError::Config("Cannot start native Claude Code".into()))
}

async fn check_claude_subscription(cwd: &PathBuf, deadline: Duration) -> Result<(), AppError> {
    let bound = limits(deadline.min(Duration::from_secs(10)));
    let output = spawn_claude(&["auth", "status"], bound, cwd)?
        .run(b"", bound)
        .await
        .map_err(|_| AppError::Auth("Claude Code subscription status unavailable".into()))?;
    if !output.status.success() {
        return Err(AppError::Auth(
            "Sign in to Claude Code with a Claude subscription".into(),
        ));
    }
    let status: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| AppError::Auth("Invalid Claude Code authentication status".into()))?;
    if status["loggedIn"] != true
        || status["authMethod"] != "claude.ai"
        || status["apiProvider"] != "firstParty"
        || status["subscriptionType"]
            .as_str()
            .is_none_or(str::is_empty)
    {
        return Err(AppError::Auth(
            "Claude Code requires a signed-in Claude subscription".into(),
        ));
    }
    Ok(())
}

fn parse_claude_final(stdout: &[u8], model: &str) -> Result<String, AppError> {
    let invalid = || AppError::Output("Claude Code returned an incomplete text response".into());
    let mut assistant_text = String::new();
    let mut final_text = None;
    for line in stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let event: Value = serde_json::from_slice(line).map_err(|_| invalid())?;
        if final_text.is_some() {
            return Err(invalid());
        }
        match event["type"].as_str() {
            Some("assistant") => {
                if event["message"]["model"] != model {
                    return Err(AppError::Output(
                        "Claude Code used a different model".into(),
                    ));
                }
                let blocks = event["message"]["content"].as_array().ok_or_else(invalid)?;
                for block in blocks {
                    match block["type"].as_str() {
                        Some("text") => {
                            assistant_text.push_str(block["text"].as_str().ok_or_else(invalid)?)
                        }
                        Some("thinking") => {}
                        _ => {
                            return Err(AppError::Output(
                                "Claude Code attempted a tool or returned non-text".into(),
                            ))
                        }
                    }
                    if assistant_text.len() > MAX_FINAL {
                        return Err(AppError::Output("Claude Code text exceeded limit".into()));
                    }
                }
            }
            Some("result") => {
                if event["subtype"] != "success"
                    || event["is_error"] != false
                    || event["stop_reason"] != "end_turn"
                    || event["terminal_reason"] != "completed"
                    || event["num_turns"] != 1
                    || event["safety_stops"] != 0
                    || event["permission_denials"]
                        .as_array()
                        .is_none_or(|items| !items.is_empty())
                {
                    return Err(invalid());
                }
                let text = event["result"].as_str().ok_or_else(invalid)?;
                if text.trim().is_empty() || text.len() > MAX_FINAL || text != assistant_text {
                    return Err(invalid());
                }
                final_text = Some(text.to_owned());
            }
            Some("system" | "rate_limit_event") => {}
            _ => return Err(invalid()),
        }
    }
    final_text.ok_or_else(invalid)
}

pub(crate) async fn complete(
    model: &str,
    content: &str,
    effort: codex::ReasoningEffort,
) -> Result<String, AppError> {
    let kind = if model.starts_with("claude-") {
        CliKind::Claude
    } else if model.starts_with("grok-") {
        CliKind::Grok
    } else {
        return Err(AppError::Config("Unsupported desktop CLI model".into()));
    };
    kind.model(model)?;
    if content.len() > MAX_PROMPT {
        return Err(AppError::Config("Desktop CLI prompt exceeded limit".into()));
    }
    if kind == CliKind::Grok {
        return Err(AppError::Config(
            "Grok CLI lacks --safe-mode and --strict-mcp-config to disable global hooks, plugins and MCP; text-only use is unavailable".into(),
        ));
    }
    let deadline = Instant::now() + DEADLINE;
    let cwd = neutral_cwd()?;
    check_claude_subscription(&cwd, deadline.saturating_duration_since(Instant::now())).await?;
    let bound = limits(deadline.saturating_duration_since(Instant::now()));
    if bound.deadline.is_zero() {
        return Err(AppError::Timeout(DEADLINE));
    }
    let output = spawn_claude(&claude_args(model, effort), bound, &cwd)?
        .run(content.as_bytes(), bound)
        .await
        .map_err(|error| AppError::Output(format!("Claude Code failed: {error}")))?;
    if !output.status.success() {
        return Err(AppError::Output(format!(
            "Claude Code exited with {}",
            output.status
        )));
    }
    parse_claude_final(&output.stdout, model)
}

pub struct ClaudeCliProvider;
pub struct GrokCliProvider;

#[async_trait]
impl LlmProvider for ClaudeCliProvider {
    async fn polish(
        &self,
        config: &LlmConfig,
        req: &PolishRequest,
        _on_chunk: Option<&ChunkCallback>,
    ) -> Result<PolishResponse, AppError> {
        CliKind::Claude.model(&config.model)?;
        let text = complete(
            &config.model,
            &codex::polish_content(req),
            codex::polish_effort(req),
        )
        .await?;
        Ok(PolishResponse {
            polished_text: codex::preserve_dictation(req, text),
        })
    }

    fn name(&self) -> &str {
        "Claude Code (subscription)"
    }
}

#[async_trait]
impl LlmProvider for GrokCliProvider {
    async fn polish(
        &self,
        config: &LlmConfig,
        req: &PolishRequest,
        _on_chunk: Option<&ChunkCallback>,
    ) -> Result<PolishResponse, AppError> {
        CliKind::Grok.model(&config.model)?;
        let text = complete(
            &config.model,
            &codex::polish_content(req),
            codex::polish_effort(req),
        )
        .await?;
        Ok(PolishResponse {
            polished_text: codex::preserve_dictation(req, text),
        })
    }

    fn name(&self) -> &str {
        "Grok CLI (subscription)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(windows)]
    #[test]
    fn windows_candidates_include_native_home_without_npm() {
        let home = PathBuf::from("C:/Users/test");
        assert_eq!(
            windows_claude_candidates(None, Some(home.clone())),
            vec![home.join(".local/bin/claude.exe")]
        );
        assert_eq!(
            windows_claude_candidates(Some(PathBuf::from("C:/AppData")), Some(home)),
            vec![
                PathBuf::from(
                    "C:/AppData/npm/node_modules/@anthropic-ai/claude-code/bin/claude.exe"
                ),
                PathBuf::from("C:/Users/test/.local/bin/claude.exe"),
            ]
        );
    }

    fn response(text: &str) -> Vec<u8> {
        let assistant = json!({
            "type": "assistant",
            "message": {
                "model": "claude-haiku-5-5",
                "content": [{"type": "text", "text": text}]
            }
        });
        let result = json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "stop_reason": "end_turn",
            "terminal_reason": "completed",
            "num_turns": 1,
            "safety_stops": 0,
            "permission_denials": [],
            "result": text
        });
        format!("{assistant}\n{result}\n").into_bytes()
    }

    #[test]
    fn claude_requires_completed_matching_text() {
        let ok = response("OK");
        assert_eq!(parse_claude_final(&ok, "claude-haiku-5-5").unwrap(), "OK");
        assert!(parse_claude_final(&ok, "claude-sonnet-5-5").is_err());
        assert!(parse_claude_final(
            &ok[..ok.iter().position(|byte| *byte == b'\n').unwrap()],
            "claude-haiku-5-5"
        )
        .is_err());
        assert!(parse_claude_final(b"not json\n", "claude-haiku-5-5").is_err());
        assert!(parse_claude_final(&response("  "), "claude-haiku-5-5").is_err());
    }

    #[test]
    fn claude_rejects_tools_refusals_and_truncation() {
        let base = String::from_utf8(response("OK")).unwrap();
        for changed in [
            base.replace("end_turn", "max_tokens"),
            base.replace("end_turn", "refusal"),
            base.replace("\"safety_stops\":0", "\"safety_stops\":1"),
            base.replace("\"num_turns\":1", "\"num_turns\":2"),
            base.replace("\"result\":\"OK\"", "\"result\":\"partial\""),
            base.replace("\"type\":\"text\"", "\"type\":\"tool_use\""),
            base.replace("\"type\":\"result\"", "\"type\":\"error\""),
            format!("{}{}", base, json!({"type": "assistant"})),
        ] {
            assert!(parse_claude_final(changed.as_bytes(), "claude-haiku-5-5").is_err());
        }
    }

    #[test]
    fn argv_disables_agent_capabilities_and_pins_models() {
        let args = claude_args("claude-haiku-5-5", codex::ReasoningEffort::Low);
        for required in [
            "--safe-mode",
            "--restricted",
            "--strict-mcp-config",
            "--tools",
            "--disallowedTools",
            "mcp__*",
            "--disable-slash-commands",
            "--permission-prompts",
            "none",
            "--no-session-persistence",
            "stream-json",
            "low",
        ] {
            assert!(args.contains(&required), "missing {required}");
        }
        assert!(args.windows(2).any(|pair| pair == ["--tools", ""]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--permission-mode", "dontAsk"]));
        assert!(args.windows(2).any(|pair| pair == ["--max-turns", "1"]));
        assert!(!args.iter().any(|arg| arg.contains("bypass")
            || arg.contains("fallback")
            || *arg == "--yolo"
            || *arg == "--always-approve"));
        assert!(CliKind::Claude.model("claude-sonnet-5-5").is_ok());
        assert!(CliKind::Grok.model("grok-4.7").is_ok());
        assert!(CliKind::Claude.model("claude-opus-5-5").is_err());
        assert!(CliKind::Grok.model("grok-build").is_err());
    }

    #[tokio::test]
    async fn grok_fails_closed_before_running_a_prompt() {
        let result = complete("grok-4.7", "Reply OK", codex::ReasoningEffort::Low).await;
        assert!(
            matches!(result, Err(AppError::Config(message)) if message.contains("--safe-mode") && message.contains("--strict-mcp-config"))
        );
    }

    #[tokio::test]
    #[ignore]
    async fn live_claude_uses_existing_subscription() {
        let text = complete(
            "claude-haiku-5-5",
            "Reply exactly OK",
            codex::ReasoningEffort::Low,
        )
        .await
        .unwrap();
        assert_eq!(text, "OK");
    }

    #[tokio::test]
    #[ignore]
    async fn live_claude_sonnet_uses_selected_model() {
        let text = complete(
            "claude-sonnet-5-5",
            "Reply exactly OK",
            codex::ReasoningEffort::Low,
        )
        .await
        .unwrap();
        assert_eq!(text, "OK");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn cancelled_cli_owner_closes_descendant_pipe() {
        use tokio::{io::AsyncReadExt, time::timeout};

        let fixture = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/process_fixture.exe");
        let bound = limits(Duration::from_secs(10));
        let mut owner = LocalProcess::spawn(fixture, bound, |command| {
            command.arg("tree");
        })
        .unwrap();
        owner.ready_line(100, bound.deadline).await.unwrap();
        owner.ready_line(100, bound.deadline).await.unwrap();
        let mut stdout = owner.take_stdout().unwrap();
        let mut waiting = Box::pin(owner.wait_owned(bound.deadline));
        assert!(matches!(
            futures_util::poll!(&mut waiting),
            std::task::Poll::Pending
        ));
        drop(waiting);
        let mut remaining = Vec::new();
        timeout(bound.deadline, stdout.read_to_end(&mut remaining))
            .await
            .unwrap()
            .unwrap();
        assert!(remaining.is_empty());
    }
}
