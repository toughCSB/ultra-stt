//! Subscription-only Codex text adapter. Only a completed final agent message is dictation.
use async_trait::async_trait;
use std::{path::{Path, PathBuf}, time::{Duration, Instant}};
use crate::{error::AppError, local_process::{Limits, LocalProcess}};
use super::{prompt, ChunkCallback, LlmConfig, LlmProvider, PolishRequest, PolishResponse};

const CODEX_EXE_RELATIVE: &str = "node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe";
const CODEX_PACKAGE_RELATIVE: &str = "npm/node_modules/@openai/codex";
const MAX_PROMPT: usize = 64 * 1024;
const MAX_FINAL: usize = 64 * 1024;
const DISABLED: &[&str] = &["shell_tool", "code_mode", "unified_exec", "browser_use", "browser_use_external", "computer_use", "view_image", "memories", "multi_agent", "hooks", "apps", "plugins"];

fn limits(deadline: u64) -> Limits {
    Limits { stdin: MAX_PROMPT, stdout: 4 * 1024 * 1024, stderr: 16 * 1024, deadline: Duration::from_secs(deadline) }
}

fn codex_package_root() -> Result<PathBuf, AppError> {
    if let Some(path) = std::env::var_os("CODEX_MANAGED_PACKAGE_ROOT") {
        return Ok(PathBuf::from(path));
    }
    let app_data = std::env::var_os("APPDATA")
        .ok_or_else(|| AppError::Config("Could not locate the current user's Codex installation".into()))?;
    Ok(PathBuf::from(app_data).join(CODEX_PACKAGE_RELATIVE))
}

fn profile_selection(value: &str) -> Result<bool, AppError> {
    match value.trim() {
        "shared" => Ok(false),
        "isolated" => Ok(true),
        _ => Err(AppError::Config("Unknown Ultra STT Codex profile; use shared or isolated".into())),
    }
}

fn selected_home() -> Result<Option<PathBuf>, AppError> {
    let root = std::env::var_os("ULTRA_STT_ROOT").map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| AppError::Config("Launch Codex integration via start-ultra-stt.ps1".into()))?;
    #[cfg(test)]
    let override_value = std::env::var("ULTRA_STT_CODEX_PROFILE").ok();
    #[cfg(not(test))]
    let override_value: Option<String> = None;
    let value = if let Some(value) = override_value { value } else {
        match std::fs::read_to_string(root.join("runtime/codex-profile")) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "shared".into(),
            Err(error) => return Err(AppError::Config(format!("Cannot read app Codex profile: {error}"))),
        }
    };
    if !profile_selection(&value)? { return Ok(None); }
    let home = root.join("runtime/codex-app-home");
    if !home.is_dir() || !home.join("model-instructions.md").is_file() {
        return Err(AppError::Config("Prepare and sign in to the Ultra STT Codex app profile before enabling it".into()));
    }
    Ok(Some(home))
}

fn spawn(args: &[&str], limits: Limits, home: Option<&Path>) -> Result<LocalProcess, AppError> {
    let package_root = codex_package_root()?;
    let exe = std::env::var_os("ULTRA_STT_CODEX_EXE").map(PathBuf::from)
        .unwrap_or_else(|| package_root.join(CODEX_EXE_RELATIVE));
    if !exe.is_file() { return Err(AppError::Config("Native Codex executable is unavailable".into())); }
    let root = std::env::var_os("ULTRA_STT_ROOT").map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| AppError::Config("Launch Codex integration via start-ultra-stt.ps1".into()))?;
    let cwd = root.join("runtime/codex");
    std::fs::create_dir_all(&cwd)
        .map_err(|e| AppError::Config(format!("Cannot prepare Codex working directory: {e}")))?;
    LocalProcess::spawn(&exe, limits, |command| {
        command.current_dir(&cwd);
        command.args(args).env("CODEX_MANAGED_PACKAGE_ROOT", &package_root);
        if let Some(home) = home { command.env("CODEX_HOME", home); }
        for name in ["OPENAI_API_KEY", "CODEX_API_KEY", "AZURE_OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_API_BASE", "AZURE_OPENAI_ENDPOINT", "CODEX_BASE_URL", "CODEX_API_BASE_URL"] { command.env_remove(name); }
    }).map_err(|e| AppError::Config(format!("Cannot start native Codex: {e}")))
}

pub async fn check_chatgpt_login() -> Result<(), AppError> {
    let home = selected_home()?;
    check_profile_login(home.as_deref()).await
}

async fn check_profile_login(home: Option<&Path>) -> Result<(), AppError> {
    let bound = limits(10);
    let output = spawn(&["login", "status"], bound, home)?.run(b"", bound).await
        .map_err(|e| AppError::Config(format!("Codex login status unavailable: {e}")))?;
    // Native Codex currently prints `login status` to stderr, unlike JSONL exec.
    let status = String::from_utf8([output.stdout, output.stderr].concat())
        .map_err(|_| AppError::Config("Invalid Codex login status".into()))?;
    if !output.status.success() || status.trim() != "Logged in using ChatGPT" {
        return Err(AppError::Auth("Codex requires an active ChatGPT subscription login (API-key login is not supported)".into()));
    }
    Ok(())
}

fn parse_final(stdout: &[u8]) -> Result<String, AppError> {
    let mut final_text: Option<String> = None;
    let mut completed = false;
    for line in stdout.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let value: serde_json::Value = serde_json::from_slice(line).map_err(|_| AppError::Config("Malformed Codex JSONL".into()))?;
        match value["type"].as_str() {
            Some("item.completed") if value["item"]["type"] == "agent_message" => {
                if completed { return Err(AppError::Config("Codex final message arrived after turn completion".into())); }
                let text = value["item"]["text"].as_str().ok_or_else(|| AppError::Config("Missing Codex final message".into()))?;
                if text.len() > MAX_FINAL { return Err(AppError::Config("Codex final message exceeded limit".into())); }
                final_text = Some(text.to_owned());
            }
            Some("turn.completed") => { completed = true; }
            Some("turn.failed") | Some("error") | Some("item.failed") => return Err(AppError::Config("Codex cleanup failed".into())),
            Some("item.started") | Some("item.updated") | Some("item.completed")
                if !matches!(value["item"]["type"].as_str(), Some("agent_message" | "reasoning")) => {
                    return Err(AppError::Config("Codex attempted a tool instead of text cleanup".into()));
                }
            _ => {}
        }
    }
    let text = final_text.filter(|text| !text.trim().is_empty());
    if !completed { return Err(AppError::Config("Codex turn did not complete".into())); }
    text.ok_or_else(|| AppError::Config("Codex returned no final text".into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReasoningEffort { Medium, High }

impl ReasoningEffort {
    fn config_override(self) -> &'static str {
        match self {
            Self::Medium => "model_reasoning_effort=\"medium\"",
            Self::High => "model_reasoning_effort=\"high\"",
        }
    }
}

fn polish_effort(req: &PolishRequest) -> ReasoningEffort {
    if req.voice_intent.kind == crate::voice_intent::VoiceIntentKind::DictateInsert
        && !req.selected_text.as_ref().is_some_and(|text| !text.trim().is_empty())
        && !req.translate_enabled
        && req.mapped_scene_prompt.trim().is_empty()
        && req.active_scene_prompt.trim().is_empty()
        && req.polish_custom_prompt.trim().is_empty()
        && req.dictionary.is_empty()
        && !req.correction_rules.iter().any(|rule| rule.enabled)
        && matches!(req.polish_style.trim(), "clean" | "minimal")
        && req.raw_text.chars().count() <= 1024
    { ReasoningEffort::Medium } else { ReasoningEffort::High }
}

fn exec_args(model: &str, effort: ReasoningEffort) -> Vec<&str> {
    let mut args = vec!["exec", "--ignore-user-config", "--ephemeral", "--sandbox", "read-only", "--skip-git-repo-check", "--color", "never", "--json", "-m", model, "-c", effort.config_override(), "-c", "service_tier=\"priority\"", "-c", "web_search=\"disabled\"", "-c", "project_doc_max_bytes=0"];
    for feature in DISABLED { args.extend(["--disable", feature]); }
    args.extend(["-c", "skills.include_instructions=false"]);
    args.push("-");
    args
}

fn profile_exec_args(model: &str, effort: ReasoningEffort, home: Option<&Path>) -> Vec<String> {
    let mut args: Vec<String> = exec_args(model, effort).into_iter().map(String::from).collect();
    if let Some(home) = home {
        args.pop();
        let path = home.join("model-instructions.md").to_string_lossy().replace('\\', "/");
        let quoted = serde_json::to_string(&path).expect("A string path is serializable");
        args.extend(["-c".into(), format!("model_instructions_file={quoted}"), "-".into()]);
    }
    args
}

pub async fn run_text(model: &str, content: &str) -> Result<String, AppError> {
    run_text_with_effort(model, content, ReasoningEffort::High).await
}

async fn run_text_with_effort(model: &str, content: &str, effort: ReasoningEffort) -> Result<String, AppError> {
    if content.len() > MAX_PROMPT { return Err(AppError::Config("Codex prompt exceeded limit".into())); }
    if model != "gpt-6-luna" { return Err(AppError::Config("Only the verified gpt-6-luna model is supported".into())); }
    let deadline = Instant::now() + Duration::from_secs(60);
    let home = selected_home()?;
    let auth_start = Instant::now();
    check_profile_login(home.as_deref()).await?;
    tracing::info!(auth_ms = auth_start.elapsed().as_millis() as u64, "Codex login preflight completed");
    let owned_args = profile_exec_args(model, effort, home.as_deref());
    let args: Vec<&str> = owned_args.iter().map(String::as_str).collect();
    let mut bound = limits(60);
    bound.deadline = deadline.saturating_duration_since(Instant::now());
    if bound.deadline.is_zero() { return Err(AppError::Config("Codex cleanup deadline exceeded".into())); }
    let exec_start = Instant::now();
    let output = spawn(&args, bound, home.as_deref())?.run(content.as_bytes(), bound).await
        .map_err(|e| AppError::Config(format!("Native Codex cleanup failed: {e}")))?;
    tracing::info!(exec_ms = exec_start.elapsed().as_millis() as u64, ?effort, service_tier = "priority", isolated_profile = home.is_some(), "Codex cleanup process completed");
    if !output.status.success() { return Err(AppError::Config(format!("Native Codex exited with {}", output.status))); }
    parse_final(&output.stdout)
}

pub struct CodexProvider;
fn polish_content(req: &PolishRequest) -> String {
    let has_selected_text = req.selected_text.as_ref().is_some_and(|s| !s.trim().is_empty());
    let system = prompt::build_context_system_prompt(prompt::ContextPromptOptions {
        context: &req.context, dictionary: &req.dictionary, correction_rules: &req.correction_rules,
        polish_style: &req.polish_style, personal_style_prompt: "", mapped_scene_prompt: &req.mapped_scene_prompt,
        active_scene_prompt: &req.active_scene_prompt, polish_custom_prompt: &req.polish_custom_prompt,
        translate_enabled: req.translate_enabled, target_lang: &req.target_lang, has_selected_text,
        voice_intent: Some(&req.voice_intent),
    });
    let selected = req.selected_text.as_deref().map(|text| format!("<selected_text>\n{text}\n</selected_text>\n")).unwrap_or_default();
    let terminology = if !req.dictionary.is_empty() || req.correction_rules.iter().any(|rule| rule.enabled) {
        "\nTERMINOLOGY PRIORITY: For a clearly matching term, the supplied dictionary's exact spelling and enabled correction rules override phonetic spelling and language-preservation rules, including Korean transliterations of English terms. Do not leave a clearly matching canonical dictionary term in phonetic form. Do not guess uncertain or unrelated names."
    } else { "" };
    format!("{system}\nRetain only the final version of an unambiguous spontaneous self-correction; preserve deliberate contrasts, negations and uncertainty.{terminology}\n{selected}<transcription>\n{}\n</transcription>", req.raw_text)
}

#[async_trait]
impl LlmProvider for CodexProvider {
    async fn polish(&self, config: &LlmConfig, req: &PolishRequest, _on_chunk: Option<&ChunkCallback>) -> Result<PolishResponse, AppError> {
        let content = polish_content(req);
        Ok(PolishResponse { polished_text: run_text_with_effort(&config.model, &content, polish_effort(req)).await? })
    }
    fn name(&self) -> &str { "Codex (ChatGPT subscription)" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_profile_selection_is_explicit_and_rejects_unknown_values() {
        assert!(!profile_selection("shared\n").unwrap());
        assert!(profile_selection("isolated\n").unwrap());
        assert!(profile_selection("unknown").is_err());
        assert!(profile_selection("").is_err());
    }

    #[test]
    fn app_profile_replaces_only_base_instructions_and_preserves_safety_flags() {
        let home = Path::new("D:/app profile/codex-app-home");
        let args = profile_exec_args("gpt-6-luna", ReasoningEffort::Medium, Some(home));
        assert!(args.windows(2).any(|pair| pair == ["-c", "model_instructions_file=\"D:/app profile/codex-app-home/model-instructions.md\""]));
        assert!(args.windows(2).any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "service_tier=\"priority\""]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "model_reasoning_effort=\"medium\""]));
        assert_eq!(args.last().map(String::as_str), Some("-"));
        assert_eq!(profile_exec_args("gpt-6-luna", ReasoningEffort::High, None), exec_args("gpt-6-luna", ReasoningEffort::High));
    }
    #[test]
    fn cleanup_excludes_unused_skill_catalog_without_weakening_quality_or_sandbox() {
        let args = exec_args("gpt-6-luna", ReasoningEffort::High);
        assert!(args.windows(2).any(|pair| pair == ["-c", "skills.include_instructions=false"]));
        assert!(args.windows(2).any(|pair| pair == ["-m", "gpt-6-luna"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "model_reasoning_effort=\"high\""]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "service_tier=\"priority\""]));
        assert!(args.windows(2).any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(args.contains(&"--ignore-user-config"));
        assert!(args.contains(&"--ephemeral"));
        assert!(!args.contains(&"skip_host_skill_discovery"));
    }

    fn dictation_request() -> PolishRequest {
        PolishRequest {
            raw_text: "맥에서 음성 입력을 테스트하고 있습니다.".into(),
            context: crate::app_detector::types::ContextProfile::general_native().summary(),
            dictionary: vec![], correction_rules: vec![], polish_style: "clean".into(),
            mapped_scene_prompt: String::new(), active_scene_prompt: String::new(),
            polish_custom_prompt: String::new(), translate_enabled: false, target_lang: "en".into(),
            selected_text: None, operation_id: None,
            voice_intent: crate::voice_intent::VoiceIntent::from_parts(
                crate::voice_intent::VoiceIntentKind::DictateInsert,
                crate::voice_intent::VoiceOutputPlacement::InsertAtCursor,
                1.0, None, None, None, None,
            ).unwrap(),
        }
    }

    #[test]
    fn terminology_priority_requires_configured_dictionary_or_enabled_corrections() {
        let mut request = dictation_request();
        assert!(!polish_content(&request).contains("TERMINOLOGY PRIORITY"));
        request.dictionary.push("Ultra STT".into());
        assert!(polish_content(&request).contains("TERMINOLOGY PRIORITY"));
        request.dictionary.clear();
        request.correction_rules.push(super::super::CorrectionRule {
            id: 1, pattern: "따이프리스".into(), replacement: "Typeless".into(), enabled: false,
        });
        assert!(!polish_content(&request).contains("TERMINOLOGY PRIORITY"));
        request.correction_rules[0].enabled = true;
        assert!(polish_content(&request).contains("TERMINOLOGY PRIORITY"));
    }

    #[test]
    fn ordinary_dictation_uses_medium_fast_and_preserves_prompt_constraints() {
        let mut request = dictation_request();
        request.selected_text = Some("  ".into());
        for style in ["clean", "minimal"] {
            request.polish_style = style.into();
            assert_eq!(polish_effort(&request), ReasoningEffort::Medium);
        }
        let args = exec_args("gpt-6-luna", polish_effort(&request));
        assert!(args.windows(2).any(|pair| pair == ["-c", "model_reasoning_effort=\"medium\""]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "service_tier=\"priority\""]));
        assert!(args.windows(2).any(|pair| pair == ["--sandbox", "read-only"]));
    }

    #[test]
    fn complex_polishing_keeps_high_reasoning() {
        let base = dictation_request();
        let mut variants = vec![];
        let mut selected = base.clone(); selected.selected_text = Some("원문".into()); variants.push(selected);
        let mut translated = base.clone(); translated.translate_enabled = true; variants.push(translated);
        let mut mapped = base.clone(); mapped.mapped_scene_prompt = "Scene".into(); variants.push(mapped);
        let mut scene = base.clone(); scene.active_scene_prompt = "Scene".into(); variants.push(scene);
        let mut custom = base.clone(); custom.polish_custom_prompt = "Custom".into(); variants.push(custom);
        let mut dictionary = base.clone(); dictionary.dictionary.push("Ultra STT".into()); variants.push(dictionary);
        let mut corrected = base.clone(); corrected.correction_rules.push(super::super::CorrectionRule {
            id: 1, pattern: "따이프리스".into(), replacement: "Typeless".into(), enabled: true,
        }); variants.push(corrected);
        let mut long = base.clone(); long.raw_text = "가".repeat(1025); variants.push(long);
        for style in ["structured", "professional", "unknown"] {
            let mut styled = base.clone(); styled.polish_style = style.into(); variants.push(styled);
        }
        for kind in [crate::voice_intent::VoiceIntentKind::DraftInsert,
            crate::voice_intent::VoiceIntentKind::RewriteSelection,
            crate::voice_intent::VoiceIntentKind::TranslateInsert,
            crate::voice_intent::VoiceIntentKind::TranslateSelection,
            crate::voice_intent::VoiceIntentKind::AskSelection,
            crate::voice_intent::VoiceIntentKind::OpenQuestion,
            crate::voice_intent::VoiceIntentKind::Search] {
            let mut special = base.clone(); special.voice_intent.kind = kind; variants.push(special);
        }
        for request in variants { assert_eq!(polish_effort(&request), ReasoningEffort::High); }
        let mut boundary = base; boundary.raw_text = "가".repeat(1024);
        assert_eq!(polish_effort(&boundary), ReasoningEffort::Medium);
    }

    #[tokio::test]
    #[ignore = "Requires shared and isolated ChatGPT logins and uses Fast quota"]
    async fn live_app_profile_paired_latency() {
        let root = PathBuf::from(std::env::var_os("ULTRA_STT_ROOT").unwrap());
        let home = root.join("runtime/codex-app-home");
        assert!(home.join("model-instructions.md").is_file());
        let request = dictation_request();
        let content = polish_content(&request);
        for repetition in 0..5 {
            let order = if repetition % 2 == 0 { [false, true] } else { [true, false] };
            for isolated in order {
                let selected = isolated.then_some(home.as_path());
                let started = Instant::now();
                check_profile_login(selected).await.unwrap();
                let owned = profile_exec_args("gpt-6-luna", polish_effort(&request), selected);
                let args: Vec<&str> = owned.iter().map(String::as_str).collect();
                let bound = limits(60);
                let output = spawn(&args, bound, selected).unwrap().run(content.as_bytes(), bound).await.unwrap();
                assert!(output.status.success());
                let final_text = parse_final(&output.stdout).unwrap();
                let compact: String = final_text.chars().filter(|character| !character.is_whitespace()).collect();
                assert_eq!(compact.trim_end_matches(['.', '。']), "맥에서음성입력을테스트하고있습니다");
                let usage = output.stdout.split(|byte| *byte == b'\n')
                    .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
                    .find(|event| event["type"] == "turn.completed").unwrap()["usage"].clone();
                println!("APP_PROFILE_PAIRED {}", serde_json::json!({
                    "repetition": repetition + 1, "profile": if isolated { "isolated" } else { "shared" },
                    "elapsed_ms": started.elapsed().as_millis() as u64, "faithful": true, "usage": usage,
                }));
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires an existing ChatGPT subscription login and uses Fast quota"]
    async fn live_hybrid_polish_preserves_synthetic_facts() {
        let config = LlmConfig { model: "gpt-6-luna".into(), ..LlmConfig::default() };
        let mut dictionary = dictation_request();
        dictionary.raw_text = "울트라 에스티티 앱으로 테스트합니다. 예약은 취소하지 마세요.".into();
        dictionary.dictionary = vec!["Ultra STT".into()];
        assert_eq!(polish_effort(&dictionary), ReasoningEffort::High);
        let dictionary_output = CodexProvider.polish(&config, &dictionary, None).await.unwrap().polished_text;
        assert!(dictionary_output.contains("Ultra STT") && dictionary_output.contains("취소하지"), "Dictionary or negation changed: {dictionary_output}");
        println!("LIVE_HYBRID case=dictionary effort=High faithful=true");
        let fixtures = [
            ("simple", "맥에서 음성 입력을 테스트하고 있습니다."),
            ("amount", "견적 금액은 십칠만 삼천오백 원 아니 십삼만 칠천오백 원입니다. 부가세는 별도입니다."),
            ("name", "김민서 팀장님 아니 최서준 팀장님께 검토 요청을 보내 주세요. 수신자는 최서준 팀장님 한 분입니다."),
            ("negation", "예약을 취소하지 마세요. 날짜만 10월 3일에서 10월 4일로 바꿔 주세요. 인원은 두 명 그대로예요."),
            ("technical", "어 B2B API v2.1 배포 전에 GitHub pull request 124번을 확인해 주세요. main 브랜치는 변경하지 마세요."),
        ];
        for (name, text) in fixtures {
            let mut request = dictation_request(); request.raw_text = text.into();
            assert_eq!(polish_effort(&request), ReasoningEffort::Medium);
            let started = Instant::now();
            let output = CodexProvider.polish(&config, &request, None).await.unwrap().polished_text;
            let compact: String = output.chars().filter(|character| !character.is_whitespace() && *character != ',').collect();
            let faithful = match name {
                "simple" => compact.trim_end_matches(['.', '。']) == "맥에서음성입력을테스트하고있습니다",
                "amount" => ["137,500", "137500", "13만7천5백", "13만7500", "십삼만칠천오백"].iter().any(|amount| compact.contains(amount))
                    && compact.contains("부가세") && compact.contains("별도") && !compact.contains("173") && !compact.contains("십칠만"),
                "name" => compact.contains("최서준") && !compact.contains("김민서") && ["한분", "1명", "한명"].iter().any(|count| compact.contains(count)),
                "negation" => compact.contains("취소하지") && compact.contains("10월4일") && ["두명", "2명"].iter().any(|count| compact.contains(count)),
                "technical" => ["B2B", "API", "v2.1", "GitHub", "124", "main", "변경하지"].iter().all(|fact| compact.contains(fact)),
                _ => false,
            };
            assert!(faithful, "Synthetic {name} facts changed: {output}");
            println!("LIVE_HYBRID case={name} effort=Medium elapsed_ms={} faithful=true", started.elapsed().as_millis());
        }
        let mut translated = dictation_request(); translated.translate_enabled = true;
        let translated_output = CodexProvider.polish(&config, &translated, None).await.unwrap().polished_text.to_lowercase();
        assert!(translated_output.contains("mac") && translated_output.contains("test")
            && (translated_output.contains("voice") || translated_output.contains("speech")), "Translation changed facts: {translated_output}");
        println!("LIVE_HYBRID case=translation effort=High faithful=true");
        let mut selected = dictation_request();
        selected.raw_text = "금액만 137,500원으로 바꿔 주세요. 부가세 별도라는 내용은 유지해 주세요.".into();
        selected.selected_text = Some("견적 금액은 173,500원입니다. 부가세는 별도입니다.".into());
        selected.voice_intent = crate::voice_intent::VoiceIntent::from_parts(
            crate::voice_intent::VoiceIntentKind::RewriteSelection,
            crate::voice_intent::VoiceOutputPlacement::ReplaceSelection,
            1.0, None, None, None, None,
        ).unwrap();
        let selected_output = CodexProvider.polish(&config, &selected, None).await.unwrap().polished_text;
        let compact = selected_output.replace([' ', ','], "");
        assert!(compact.contains("137500") && !compact.contains("173500")
            && compact.contains("부가세") && compact.contains("별도"), "Selected edit changed facts: {selected_output}");
        println!("LIVE_HYBRID case=selected-edit effort=High faithful=true");
        let mut corrected = dictation_request();
        corrected.raw_text = "따이프리스보다 빠를지는 아직 모르겠습니다. 예약은 취소하지 마세요.".into();
        corrected.correction_rules = vec![super::super::CorrectionRule {
            id: 1, pattern: "따이프리스".into(), replacement: "Typeless".into(), enabled: true,
        }];
        let corrected_output = CodexProvider.polish(&config, &corrected, None).await.unwrap().polished_text;
        assert!(corrected_output.contains("Typeless") && !corrected_output.contains("따이프리스")
            && corrected_output.contains("모르") && corrected_output.contains("취소하지"), "Correction or uncertainty changed: {corrected_output}");
        assert_eq!(polish_effort(&corrected), ReasoningEffort::High);
        println!("LIVE_HYBRID case=correction-uncertainty effort=High faithful=true");
        let mut structured = dictation_request();
        structured.polish_style = "structured".into();
        structured.raw_text = "할 일은 세 가지입니다. 첫째 계약서 검토, 둘째 김포 지점 전화, 셋째 영수증 세 장 정리입니다.".into();
        let structured_output = CodexProvider.polish(&config, &structured, None).await.unwrap().polished_text;
        assert!(["계약서", "김포", "전화", "영수증"].iter().all(|fact| structured_output.contains(fact))
            && (structured_output.contains("세 장") || structured_output.contains("3장") || structured_output.contains("3 장")), "Structured facts changed: {structured_output}");
        println!("LIVE_HYBRID case=structured effort=High faithful=true");
        let answer = run_text("gpt-6-luna", "This is an explicit question operation, not dictation. What is 12 multiplied by 3? Return only the integer.").await.unwrap();
        assert_eq!(answer.trim(), "36");
        println!("LIVE_HYBRID case=question effort=High faithful=true");
    }

    #[test]
    fn final_only_requires_completed_turn() {
        let json = br#"{"type":"item.completed","item":{"type":"agent_message","text":"final"}}
{"type":"turn.completed"}
"#;
        assert_eq!(parse_final(json).unwrap(), "final");
        assert!(parse_final(&json[..json.len()-26]).is_err());
        assert!(parse_final(br#"{"type":"item.completed","item":{"type":"reasoning","text":"private"}}
{"type":"turn.completed"}
"#).is_err());
        assert!(parse_final(br#"{"type":"turn.failed"}
"#).is_err());
        assert!(parse_final(br#"{"type":"item.completed","item":{"type":"command_execution","text":"tool output"}}
{"type":"turn.completed"}
"#).is_err());
    }
}
