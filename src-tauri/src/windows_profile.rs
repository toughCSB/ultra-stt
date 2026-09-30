use std::path::{Path, PathBuf};

fn packaged_root(executable: &Path) -> Result<Option<PathBuf>, String> {
    let marker = executable
        .parent()
        .ok_or("Executable has no parent")?
        .join("server-root.txt");
    let content = match std::fs::read_to_string(marker) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read personal runtime location: {error}")),
    };
    if content.trim() == "@local-runtime@" {
        return Ok(None);
    }
    let root = PathBuf::from(content.trim());
    if !root.is_absolute() {
        return Err("Personal runtime location must be absolute".into());
    }
    for required in [
        ".venv/Scripts/python.exe",
        "models/large-v3/model.bin",
        "opentypeless/local_stt/server.py",
        "data/settings.json",
    ] {
        if !root.join(required).is_file() {
            return Err(format!("Personal runtime is incomplete: {required}"));
        }
    }
    Ok(Some(root))
}

#[cfg(target_os = "windows")]
pub fn configure() -> Result<(), String> {
    if std::env::var_os("ULTRA_STT_ROOT").is_some()
        || std::env::var_os("ULTRA_STT_DATA_DIR").is_some()
    {
        return Ok(());
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let Some(root) = packaged_root(&executable)? else {
        return Ok(());
    };
    std::env::set_var("ULTRA_STT_DATA_DIR", root.join("data"));
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", root.join("webview"));
    std::env::set_var("CUDA_CACHE_PATH", root.join("cache/cuda"));
    std::env::set_var("ULTRA_STT_ROOT", root);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_app_resolves_existing_profile_without_recreating_user_data() {
        let temporary =
            std::env::temp_dir().join(format!("ultra-profile-{}", uuid::Uuid::new_v4()));
        let install = temporary.join("installed");
        let root = temporary.join("runtime with spaces");
        std::fs::create_dir_all(&install).unwrap();
        let executable = install.join("Ultra_STT.exe");
        assert_eq!(packaged_root(&executable).unwrap(), None);
        let marker = install.join("server-root.txt");
        std::fs::write(&marker, "@local-runtime@").unwrap();
        assert_eq!(packaged_root(&executable).unwrap(), None);
        std::fs::write(&marker, "relative/runtime").unwrap();
        assert!(packaged_root(&executable).is_err());
        std::fs::write(&marker, format!("{}\n", root.display())).unwrap();
        assert!(packaged_root(&executable).is_err());
        for required in [
            ".venv/Scripts/python.exe",
            "models/large-v3/model.bin",
            "opentypeless/local_stt/server.py",
            "data/settings.json",
        ] {
            let path = root.join(required);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "fixture").unwrap();
        }
        assert_eq!(packaged_root(&executable).unwrap(), Some(root.clone()));
        assert_eq!(
            std::fs::read_to_string(root.join("data/settings.json")).unwrap(),
            "fixture"
        );
        std::fs::remove_file(root.join("models/large-v3/model.bin")).unwrap();
        assert!(packaged_root(&executable).is_err());
        std::fs::remove_dir_all(temporary).unwrap();
    }
}
