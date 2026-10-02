use crate::{pipeline, storage};
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::Manager;
use tauri_plugin_store::StoreExt;

pub fn should_toggle_on_tray_event(event: &tauri::tray::TrayIconEvent) -> bool {
    matches!(event, tauri::tray::TrayIconEvent::Click {
        button: tauri::tray::MouseButton::Left,
        button_state: tauri::tray::MouseButtonState::Up,
        ..
    })
}

fn should_hide_main_window(visible: bool, minimized: bool) -> bool {
    visible && !minimized
}

pub fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let hide = should_hide_main_window(
            window.is_visible().unwrap_or(false), window.is_minimized().unwrap_or(false),
        );
        let result = if hide {
            window.hide()
        } else {
            window.unminimize().and_then(|_| window.show()).and_then(|_| window.set_focus())
        };
        match result {
            Ok(()) => tracing::info!(target_visible = !hide, "Tray window toggle requested"),
            Err(error) => tracing::warn!("Tray window toggle failed: {error}"),
        }
        refresh_tray(app);
    }
}

/// Managed tray icon handle for dynamic menu/tooltip updates.
pub struct TrayHandle {
    pub tray: Mutex<tauri::tray::TrayIcon>,
}

struct TrayLabels {
    show_window: &'static str,
    hide_window: &'static str,
    start_recording: &'static str,
    stop_recording: &'static str,
    hide_capsule_when_idle: &'static str,
    keep_capsule_visible: &'static str,
    settings: &'static str,
    history: &'static str,
    account: &'static str,
    about: &'static str,
    quit: &'static str,
}

fn get_tray_labels(lang: &str) -> TrayLabels {
    match lang {
        "zh" => TrayLabels {
            show_window: "显示窗口",
            hide_window: "隐藏窗口",
            start_recording: "开始录音",
            stop_recording: "停止录音",
            hide_capsule_when_idle: "空闲时隐藏胶囊",
            keep_capsule_visible: "保持胶囊可见",
            settings: "设置",
            history: "历史记录",
            account: "账户",
            about: "关于 Ultra STT",
            quit: "退出",
        },
        "ja" => TrayLabels {
            show_window: "ウィンドウを表示",
            hide_window: "ウィンドウを隠す",
            start_recording: "録音を開始",
            stop_recording: "録音を停止",
            hide_capsule_when_idle: "待機中はカプセルを非表示",
            keep_capsule_visible: "カプセルを表示したままにする",
            settings: "設定",
            history: "履歴",
            account: "アカウント",
            about: "Ultra STT について",
            quit: "終了",
        },
        "ko" => TrayLabels {
            show_window: "창 보이기",
            hide_window: "창 숨기기",
            start_recording: "녹음 시작",
            stop_recording: "녹음 중지",
            hide_capsule_when_idle: "유휴 시 캡슐 숨기기",
            keep_capsule_visible: "캡슐 항상 표시",
            settings: "설정",
            history: "기록",
            account: "계정",
            about: "Ultra STT 정보",
            quit: "종료",
        },
        "fr" => TrayLabels {
            show_window: "Afficher la fenêtre",
            hide_window: "Masquer la fenêtre",
            start_recording: "Démarrer l'enregistrement",
            stop_recording: "Arrêter l'enregistrement",
            hide_capsule_when_idle: "Masquer la capsule au repos",
            keep_capsule_visible: "Garder la capsule visible",
            settings: "Paramètres",
            history: "Historique",
            account: "Compte",
            about: "À propos d'Ultra STT",
            quit: "Quitter",
        },
        "de" => TrayLabels {
            show_window: "Fenster anzeigen",
            hide_window: "Fenster ausblenden",
            start_recording: "Aufnahme starten",
            stop_recording: "Aufnahme stoppen",
            hide_capsule_when_idle: "Kapsel im Leerlauf ausblenden",
            keep_capsule_visible: "Kapsel sichtbar lassen",
            settings: "Einstellungen",
            history: "Verlauf",
            account: "Konto",
            about: "Über Ultra STT",
            quit: "Beenden",
        },
        "es" => TrayLabels {
            show_window: "Mostrar ventana",
            hide_window: "Ocultar ventana",
            start_recording: "Iniciar grabación",
            stop_recording: "Detener grabación",
            hide_capsule_when_idle: "Ocultar cápsula en reposo",
            keep_capsule_visible: "Mantener cápsula visible",
            settings: "Configuración",
            history: "Historial",
            account: "Cuenta",
            about: "Acerca de Ultra STT",
            quit: "Salir",
        },
        "pt" => TrayLabels {
            show_window: "Mostrar janela",
            hide_window: "Ocultar janela",
            start_recording: "Iniciar gravação",
            stop_recording: "Parar gravação",
            hide_capsule_when_idle: "Ocultar cápsula em repouso",
            keep_capsule_visible: "Manter cápsula visível",
            settings: "Configurações",
            history: "Histórico",
            account: "Conta",
            about: "Sobre o Ultra STT",
            quit: "Sair",
        },
        "ru" => TrayLabels {
            show_window: "Показать окно",
            hide_window: "Скрыть окно",
            start_recording: "Начать запись",
            stop_recording: "Остановить запись",
            hide_capsule_when_idle: "Скрывать капсулу в простое",
            keep_capsule_visible: "Оставлять капсулу видимой",
            settings: "Настройки",
            history: "История",
            account: "Аккаунт",
            about: "О программе Ultra STT",
            quit: "Выход",
        },
        "it" => TrayLabels {
            show_window: "Mostra finestra",
            hide_window: "Nascondi finestra",
            start_recording: "Inizia registrazione",
            stop_recording: "Ferma registrazione",
            hide_capsule_when_idle: "Nascondi capsula quando inattiva",
            keep_capsule_visible: "Mantieni capsula visibile",
            settings: "Impostazioni",
            history: "Cronologia",
            account: "Account",
            about: "Informazioni su Ultra STT",
            quit: "Esci",
        },
        _ => TrayLabels {
            show_window: "Show Window",
            hide_window: "Hide Window",
            start_recording: "Start Recording",
            stop_recording: "Stop Recording",
            hide_capsule_when_idle: "Hide Capsule When Idle",
            keep_capsule_visible: "Keep Capsule Visible",
            settings: "Settings",
            history: "History",
            account: "Account",
            about: "About Ultra STT",
            quit: "Quit",
        },
    }
}

fn app_config_value(app: &tauri::AppHandle) -> Option<serde_json::Value> {
    app.store(storage::profile_store_path("settings.json"))
        .ok()
        .and_then(|store| store.get("app_config"))
}

fn tray_language(app: &tauri::AppHandle) -> String {
    app_config_value(app)
        .and_then(|config| {
            config
                .get("ui_language")
                .and_then(|value| value.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "en".to_string())
}

fn capsule_auto_hide_from_app_config(config: Option<&serde_json::Value>) -> bool {
    match config {
        Some(config) => config
            .get("capsule_auto_hide")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        None => storage::AppConfig::new_install_default().capsule_auto_hide,
    }
}

fn tray_capsule_auto_hide(app: &tauri::AppHandle) -> bool {
    let config = app_config_value(app);
    capsule_auto_hide_from_app_config(config.as_ref())
}

fn polish_mode_labels(lang: &str) -> (&'static str, [&'static str; 5]) {
    match lang {
        "ko" => ("AI 다듬기 모드", ["원문 · Original", "최소", "깔끔하게", "구조화", "전문적으로"]),
        "ja" => ("整形スタイル", ["原文", "最小", "クリーン", "構造化", "プロ"]),
        "zh" => ("润色风格", ["原文", "轻改", "清爽", "结构化", "专业"]),
        "de" => ("Schreibstil", ["Original", "Minimal", "Klar", "Strukturiert", "Professionell"]),
        "es" => ("Estilo de pulido", ["Original", "Mínimo", "Limpio", "Estructurado", "Profesional"]),
        "fr" => ("Style de réécriture", ["Original", "Minimal", "Clair", "Structuré", "Professionnel"]),
        "it" => ("Stile di rifinitura", ["Originale", "Minimo", "Pulito", "Strutturato", "Professionale"]),
        "pt" => ("Estilo de polimento", ["Original", "Mínimo", "Limpo", "Estruturado", "Profissional"]),
        "ru" => ("Стиль обработки", ["Оригинал", "Минимальный", "Чистый", "Структурный", "Профессиональный"]),
        _ => ("AI polish mode", ["Original", "Minimal", "Clean", "Structured", "Professional"]),
    }
}

/// Build (or rebuild) the system tray menu based on current state.
pub fn build_tray_menu(
    app: &tauri::AppHandle,
    is_recording: bool,
    window_visible: bool,
    capsule_auto_hide: bool,
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let lang = tray_language(app);
    let labels = get_tray_labels(&lang);
    let (mode_label, mode_labels) = polish_mode_labels(&lang);
    let config = app_config_value(app);
    let current_style = config.as_ref().and_then(|value| value.get("polish_style"))
        .and_then(|value| value.as_str()).unwrap_or("clean");
    let polish_modes = Submenu::new(app, mode_label, true)?;
    for (index, style) in storage::POLISH_STYLES.iter().enumerate() {
        let item = CheckMenuItem::with_id(app, format!("polish_style:{style}"), mode_labels[index], true, current_style == *style, None::<&str>)?;
        polish_modes.append(&item)?;
    }

    let show_hide = MenuItem::with_id(
        app,
        "show_hide",
        if window_visible {
            labels.hide_window
        } else {
            labels.show_window
        },
        true,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let record = MenuItem::with_id(
        app,
        "record",
        if is_recording {
            labels.stop_recording
        } else {
            labels.start_recording
        },
        true,
        None::<&str>,
    )?;
    let capsule_visibility = MenuItem::with_id(
        app,
        "toggle_capsule_auto_hide",
        if capsule_auto_hide {
            labels.keep_capsule_visible
        } else {
            labels.hide_capsule_when_idle
        },
        true,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", labels.settings, true, None::<&str>)?;
    let history = MenuItem::with_id(app, "history", labels.history, true, None::<&str>)?;
    let account = MenuItem::with_id(app, "account", labels.account, true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let about = MenuItem::with_id(app, "about", labels.about, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_hide,
            &sep1,
            &record,
            &capsule_visibility,
            &polish_modes,
            &sep2,
            &settings,
            &history,
            &account,
            &sep3,
            &about,
            &quit,
        ],
    )?;
    Ok(menu)
}

/// Rebuild the tray menu and update tooltip based on pipeline state.
pub fn refresh_tray(app: &tauri::AppHandle) {
    let is_recording = app
        .try_state::<pipeline::PipelineHandle>()
        .map(|p| p.current_state() == pipeline::PipelineState::Recording)
        .unwrap_or(false);
    let window_visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    let capsule_auto_hide = tray_capsule_auto_hide(app);

    if let Some(tray_handle) = app.try_state::<TrayHandle>() {
        if let Ok(tray) = tray_handle.tray.lock() {
            if let Ok(menu) = build_tray_menu(app, is_recording, window_visible, capsule_auto_hide)
            {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_click_hides_visible_window_and_restores_hidden_or_minimized_window() {
        assert!(should_hide_main_window(true, false));
        assert!(!should_hide_main_window(false, false));
        assert!(!should_hide_main_window(true, true));
        assert!(!should_hide_main_window(false, true));
    }

    #[test]
    fn tray_only_toggles_once_per_left_release_and_ignores_double_click_notification() {
        use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
        let rect = tauri::Rect { position: tauri::PhysicalPosition::new(0, 0).into(), size: tauri::PhysicalSize::new(24, 24).into() };
        for (button, state, expected) in [
            (MouseButton::Left, MouseButtonState::Up, true),
            (MouseButton::Left, MouseButtonState::Down, false),
            (MouseButton::Right, MouseButtonState::Up, false),
            (MouseButton::Middle, MouseButtonState::Up, false),
        ] {
            let event = TrayIconEvent::Click { id: "test".into(), position: tauri::PhysicalPosition::new(0.0, 0.0), rect, button, button_state: state };
            assert_eq!(should_toggle_on_tray_event(&event), expected);
        }
        let double = TrayIconEvent::DoubleClick { id: "test".into(), position: tauri::PhysicalPosition::new(0.0, 0.0), rect, button: MouseButton::Left };
        assert!(!should_toggle_on_tray_event(&double));
    }

    #[test]
    fn tray_icon_is_an_opaque_black_tile_with_brand_green_microphone() {
        let pixels = include_bytes!("../icons/tray-icon.rgba");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
        for offset in [0, 63 * 4, 63 * 64 * 4, (64 * 64 - 1) * 4] {
            assert!(pixels[offset..offset + 3].iter().all(|channel| *channel <= 5));
        }
        assert!(pixels.chunks_exact(4).filter(|pixel| pixel[1] > 130 && pixel[1] > pixel[0].saturating_add(40)).count() > 600);
    }

    #[test]
    fn tray_labels_include_capsule_visibility_controls() {
        let labels = get_tray_labels("en");

        assert_eq!(labels.hide_capsule_when_idle, "Hide Capsule When Idle");
        assert_eq!(labels.keep_capsule_visible, "Keep Capsule Visible");
    }

    #[test]
    fn missing_app_config_uses_new_install_capsule_auto_hide_default() {
        assert!(capsule_auto_hide_from_app_config(None));
    }

    #[test]
    fn existing_app_config_missing_capsule_auto_hide_preserves_legacy_visible_default() {
        let config = serde_json::json!({});

        assert!(!capsule_auto_hide_from_app_config(Some(&config)));
    }
}
