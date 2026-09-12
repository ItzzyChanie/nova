//! Fail closed on corrupt persisted JSON instead of the plugin's ignored load errors.
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::{Manager, Runtime};
use tauri_plugin_store::{Store, StoreBuilder, StoreExt};
static LOAD_LOCK: Mutex<()> = Mutex::new(());
fn read_values(path: &Path) -> Result<HashMap<String, Value>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| "Local NOVA data is invalid JSON. Quit NOVA and restore this store from a backup; the original file was preserved.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(_) => Err("Local NOVA data cannot be read. Check application-data permissions before retrying; the file was preserved.".into()),
    }
}
pub trait SafeStoreExt<R: Runtime>: Manager<R> + Sized {
    fn store(&self, path: impl AsRef<Path>) -> Result<Arc<Store<R>>, String> {
        let _guard = LOAD_LOCK
            .lock()
            .map_err(|_| "Local data is busy.".to_string())?;
        let path = path.as_ref();
        if let Some(store) = StoreExt::get_store(self, path) {
            return Ok(store);
        }
        let directory = self
            .app_handle()
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;
        let values = read_values(&directory.join(path)).map_err(|error| {
            crate::local_log::event("storage", "load_failed");
            format!("{}: {error}", path.display())
        })?;
        // Seed with the exact validated data, avoiding a second, error-ignoring file load.
        StoreBuilder::new(self.app_handle(), path)
            .defaults(values)
            .create_new()
            .build()
            .map_err(|e| e.to_string())
    }
}
impl<R: Runtime, T: Manager<R>> SafeStoreExt<R> for T {}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_store_is_rejected_without_modifying_it() {
        let path =
            std::env::temp_dir().join(format!("nova-corrupt-store-{}.json", std::process::id()));
        for bytes in [b"{broken".as_slice(), b"[]".as_slice()] {
            std::fs::write(&path, bytes).unwrap();
            assert!(read_values(&path).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        std::fs::write(&path, br#"{"novaEnabled":false}"#).unwrap();
        assert_eq!(read_values(&path).unwrap()["novaEnabled"], false);
        std::fs::remove_file(&path).unwrap();
        assert!(read_values(&path).unwrap().is_empty());
    }
}
