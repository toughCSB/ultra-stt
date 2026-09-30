//! Exercise production adapters without opening a window. The PCM is a public fixture.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("speech") => {
            let file = std::env::args().nth(2).ok_or("expected PCM fixture path")?;
            let pcm = std::fs::read(file)?;
            let mut provider = opentypeless_lib::stt::ultra_local::UltraLocalProvider::new();
            use opentypeless_lib::stt::SttProvider;
            provider.connect(&opentypeless_lib::stt::SttConfig {
                language: Some("ko".into()), ..Default::default()
            }).await?;
            provider.send_audio(&pcm).await?;
            println!("Korean fixture: {:?}", provider.disconnect().await?);
            let silence = vec![0; 64000];
            println!("Silence: {:?}", opentypeless_lib::stt::ultra_local::transcribe(&silence, Some("ko")).await?);
            opentypeless_lib::stt::ultra_local::shutdown().await;
        }
        Some("codex") => {
            use opentypeless_lib::{app_detector::types::ContextProfile, llm::{LlmProvider, PolishRequest, LlmConfig}, voice_intent::{VoiceIntent, VoiceIntentKind, VoiceOutputPlacement}};
            let request = PolishRequest {
                raw_text: "회의는 화요일 아니 수요일 2026년 9월 30일입니다".into(),
                context: ContextProfile::general_native().summary(),
                dictionary: vec![], correction_rules: vec![], polish_style: "clean".into(),
                mapped_scene_prompt: String::new(), active_scene_prompt: String::new(),
                polish_custom_prompt: String::new(), translate_enabled: false, target_lang: "ko".into(),
                selected_text: None, operation_id: None,
                voice_intent: VoiceIntent::from_parts(VoiceIntentKind::DictateInsert,
                    VoiceOutputPlacement::InsertAtCursor, 1.0, None, None, None, None)?,
            };
            let result = opentypeless_lib::llm::codex::CodexProvider.polish(
                &LlmConfig { provider: "codex-cli".into(), api_key: String::new(), model: "gpt-6-luna".into(),
                    base_url: String::new(), max_tokens: 4096, temperature: 0.3 }, &request, None).await?;
            println!("Codex final: {}", result.polished_text);
        }
        _ => return Err("usage: native_adapters speech <pcm-file> | codex".into()),
    }
    Ok(())
}
