//! Only compile-time event labels are accepted: no transcripts, paths or error payloads.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());
const LIMIT: u64 = 1024 * 1024;

pub fn directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|p| p.join("logs"))
        .map_err(|e| e.to_string())
}
pub fn setup(app: &AppHandle) -> Result<(), String> {
    let path = directory(app)?;
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.join("nova.log"))
        .map_err(|e| e.to_string())?;
    let _ = DIRECTORY.set(path);
    event("runtime", "startup");
    std::panic::set_hook(Box::new(|_| event("runtime", "panic")));
    Ok(())
}
fn append(directory: &Path, line: &str) -> std::io::Result<()> {
    let path = directory.join("nova.log");
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) + line.len() as u64 > LIMIT {
        let previous = directory.join("nova.previous.log");
        match fs::remove_file(&previous) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        fs::rename(&path, previous)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}
pub fn event(component: &'static str, state: &'static str) {
    let Some(directory) = DIRECTORY.get() else {
        return;
    };
    let Ok(_guard) = LOCK.lock() else { return };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let _ = append(directory, &format!("{timestamp} {component} {state}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_retains_only_two_bounded_files() {
        let directory = std::env::temp_dir().join(format!("nova-log-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let line = "x".repeat(1024);
        for _ in 0..2050 {
            append(&directory, &line).unwrap();
        }
        assert!(fs::metadata(directory.join("nova.log")).unwrap().len() <= LIMIT);
        assert!(
            fs::metadata(directory.join("nova.previous.log"))
                .unwrap()
                .len()
                <= LIMIT
        );
        fs::remove_file(directory.join("nova.log")).unwrap();
        fs::remove_file(directory.join("nova.previous.log")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
