use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Source-tree resources are a debug-only convenience, never a release fallback.
pub fn resource(app: &AppHandle, relative: &str) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Could not locate NOVA resources: {e}"))?
        .join(relative);
    #[cfg(debug_assertions)]
    if !bundled.exists() {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative));
    }
    Ok(bundled)
}
