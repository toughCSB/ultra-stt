use tauri_plugin_autostart::ManagerExt;

pub(crate) struct StartupScope(pub bool);

impl StartupScope {
    pub(crate) fn from_overrides(data_override: bool, root_override: bool) -> Self {
        Self(!data_override && !root_override)
    }
}

pub(crate) fn apply(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|error| error.to_string())?;
        #[cfg(windows)]
        write_windows_launch_path(app)?;
    } else {
        #[cfg(windows)]
        remove_windows_launch_path(app)?;
        #[cfg(not(windows))]
        if autolaunch.is_enabled().map_err(|error| error.to_string())? {
            autolaunch.disable().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn quoted_executable(path: &std::path::Path) -> Result<String, String> {
    let path = path
        .to_str()
        .ok_or("Startup executable path is not valid Unicode")?;
    if path.is_empty() || path.contains(['"', '\0', '\r', '\n']) {
        return Err("Invalid startup executable path".to_string());
    }
    Ok(format!("\"{path}\""))
}

#[cfg(windows)]
fn remove_windows_launch_path(app: &tauri::AppHandle) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER},
    };
    let name: Vec<u16> = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or(&app.package_info().name)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            windows_sys::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            name.as_ptr(),
        )
    };
    match result {
        ERROR_SUCCESS | ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(()),
        _ => Err(format!(
            "Failed to disable startup: {}",
            std::io::Error::from_raw_os_error(result as i32)
        )),
    }
}

#[cfg(windows)]
fn write_windows_launch_path(app: &tauri::AppHandle) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let command: Vec<u16> = quoted_executable(&executable)?
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or(&app.package_info().name)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            windows_sys::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            name.as_ptr(),
            REG_SZ,
            command.as_ptr().cast(),
            (command.len() * std::mem::size_of::<u16>()) as u32,
        )
    };
    if result != 0 {
        return Err(format!(
            "Failed to set startup executable path: {}",
            std::io::Error::from_raw_os_error(result as i32)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_profiles_allow_startup_but_external_test_overrides_do_not() {
        assert!(StartupScope::from_overrides(false, false).0);
        assert!(!StartupScope::from_overrides(true, false).0);
        assert!(!StartupScope::from_overrides(false, true).0);
        assert!(!StartupScope::from_overrides(true, true).0);
    }

    #[test]
    fn startup_path_quotes_spaces_and_preserves_unicode() {
        assert_eq!(
            quoted_executable(std::path::Path::new(r"C:\Users\떡배\My Apps\Ultra_STT.exe"))
                .unwrap(),
            "\"C:\\Users\\떡배\\My Apps\\Ultra_STT.exe\""
        );
    }

    #[test]
    fn startup_path_rejects_invalid_command_characters() {
        for path in ["", "app\".exe", "app\0.exe", "app\n.exe"] {
            assert!(quoted_executable(std::path::Path::new(path)).is_err());
        }
    }
}
