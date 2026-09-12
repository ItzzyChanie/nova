use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    ffi::OsStr,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use crate::local_store::SafeStoreExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

const PROJECTS_FILE: &str = "nova-projects.json";
const PROJECTS_KEY: &str = "projects";
const MAX_RESULTS: usize = 50;
const MAX_VISITED: usize = 10_000;
const MAX_DEPTH: usize = 6;

static SEARCH_RESULTS: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FileToolErrorCode {
    UnsupportedPlatform,
    UnknownRoot,
    OutsideApprovedRoots,
    FileNotFound,
    FolderNotFound,
    InvalidResultId,
    InvalidProject,
    SearchFailed,
    OpenFailed,
    RevealFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileToolError {
    pub code: FileToolErrorCode,
    pub message: String,
}

impl FileToolError {
    fn new(code: FileToolErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectRecord {
    #[serde(default)]
    pub voice_enabled: bool,
    #[serde(default)]
    pub frontend_command: String,
    #[serde(default)]
    pub backend_command: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub development_url: String,
    #[serde(default)]
    pub notes: String,
    pub id: String,
    pub name: String,
    pub path: String,
    pub editor: String,
}

#[derive(Debug, Clone)]
struct ApprovedRoot {
    id: String,
    name: String,
    path: PathBuf,
    kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchCandidate {
    result_id: String,
    name: String,
    path: String,
    kind: &'static str,
    extension: Option<String>,
    modified_at: Option<u64>,
    relevance: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchArguments {
    pub query: String,
    pub root: Option<String>,
    pub extension: Option<String>,
    pub exact: Option<bool>,
    pub modified_within_days: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileTargetArguments {
    pub path: Option<String>,
    pub result_id: Option<String>,
}

fn required_text(value: &str, field: &str) -> Result<(), FileToolError> {
    if value.trim().is_empty() {
        return Err(FileToolError::new(
            FileToolErrorCode::SearchFailed,
            format!("'{field}' must be a non-empty string."),
        ));
    }
    if value.len() > 512 {
        return Err(FileToolError::new(
            FileToolErrorCode::SearchFailed,
            format!("'{field}' exceeds the 512-character limit."),
        ));
    }
    Ok(())
}

fn user_root(name: &str) -> Option<PathBuf> {
    env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(name))
}

fn project_id(path: &Path) -> String {
    format!("project-{:016x}", stable_hash(path))
}

fn stable_hash(path: &Path) -> u64 {
    struct Fnv(u64);
    impl Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for byte in bytes {
                self.0 ^= *byte as u64;
                self.0 = self.0.wrapping_mul(0x100000001b3);
            }
        }
    }
    let mut hasher = Fnv(0xcbf29ce484222325);
    path.to_string_lossy().to_lowercase().hash(&mut hasher);
    hasher.finish()
}

fn result_id(path: &Path, kind: &str) -> String {
    format!("{kind}-{:016x}", stable_hash(path))
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(stripped) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{stripped}");
    }
    value
        .strip_prefix(r"\\?\")
        .unwrap_or(value.as_ref())
        .to_string()
}

fn projects(app: &AppHandle) -> Result<Vec<ProjectRecord>, FileToolError> {
    let store = app.store(PROJECTS_FILE).map_err(|error| {
        FileToolError::new(
            FileToolErrorCode::SearchFailed,
            format!("Could not open project settings: {error}"),
        )
    })?;
    let mut records: Vec<ProjectRecord> = match store.get(PROJECTS_KEY) {
        None => Vec::new(),
        Some(value) => serde_json::from_value(value).map_err(|e| FileToolError::new(FileToolErrorCode::InvalidProject, format!("Project data is invalid: {e}")))?,
    };
    for record in &mut records {
        record.path = display_path(Path::new(&record.path));
    }
    Ok(records)
}

fn approved_roots(app: &AppHandle) -> Result<Vec<ApprovedRoot>, FileToolError> {
    let mut roots = Vec::new();
    for (id, name) in [
        ("desktop", "Desktop"),
        ("documents", "Documents"),
        ("downloads", "Downloads"),
        ("videos", "Videos"),
        ("music", "Music"),
        ("projects", "Projects"),
    ] {
        if let Some(path) = user_root(name) {
            if let Ok(canonical) = path.canonicalize() {
                roots.push(ApprovedRoot {
                    id: id.into(),
                    name: name.into(),
                    path: canonical,
                    kind: "knownFolder",
                });
            }
        }
    }

    for project in projects(app)? {
        let path = PathBuf::from(&project.path);
        if let Ok(canonical) = path.canonicalize() {
            roots.push(ApprovedRoot {
                id: project.id,
                name: project.name,
                path: canonical,
                kind: "project",
            });
        }
    }

    let mut seen = HashSet::new();
    roots.retain(|root| seen.insert(root.path.clone()));
    Ok(roots)
}

fn resolve_root(app: &AppHandle, requested: &str) -> Result<ApprovedRoot, FileToolError> {
    let normalized = requested.trim().to_lowercase();
    let matches: Vec<_> = approved_roots(app)?
        .into_iter()
        .filter(|root| {
            root.id.to_lowercase() == normalized
                || root.name.to_lowercase() == normalized
                || root.path.to_string_lossy().to_lowercase() == normalized
        })
        .collect();
    match matches.as_slice() {
        [] => Err(FileToolError::new(
            FileToolErrorCode::UnknownRoot,
            format!(
                "'{requested}' is not an approved root. Use Desktop, Documents, Downloads, Videos, Music, Projects, or a registered project."
            ),
        )),
        [root] => Ok(root.clone()),
        _ => Err(FileToolError::new(
            FileToolErrorCode::UnknownRoot,
            format!("'{requested}' matches more than one approved root."),
        )),
    }
}

fn approved_path(
    app: &AppHandle,
    requested: &Path,
    expect_file: bool,
) -> Result<PathBuf, FileToolError> {
    let canonical = requested.canonicalize().map_err(|_| {
        FileToolError::new(
            if expect_file {
                FileToolErrorCode::FileNotFound
            } else {
                FileToolErrorCode::FolderNotFound
            },
            format!("'{}' does not exist.", requested.display()),
        )
    })?;
    let roots = approved_roots(app)?;
    if !roots.iter().any(|root| canonical.starts_with(&root.path)) {
        return Err(FileToolError::new(
            FileToolErrorCode::OutsideApprovedRoots,
            "The requested path is outside NOVA's approved roots.",
        ));
    }
    if expect_file && !canonical.is_file() {
        return Err(FileToolError::new(
            FileToolErrorCode::FileNotFound,
            "The requested path is not a file.",
        ));
    }
    if !expect_file && !canonical.is_dir() {
        return Err(FileToolError::new(
            FileToolErrorCode::FolderNotFound,
            "The requested path is not a folder.",
        ));
    }
    Ok(canonical)
}

fn cache_candidate(path: &Path, kind: &str) -> String {
    let id = result_id(path, kind);
    SEARCH_RESULTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(id.clone(), path.to_path_buf());
    id
}

fn resolve_target(
    app: &AppHandle,
    arguments: &FileTargetArguments,
    expect_file: bool,
) -> Result<PathBuf, FileToolError> {
    match (&arguments.path, &arguments.result_id) {
        (Some(path), None) => approved_path(app, Path::new(path), expect_file),
        (None, Some(id)) => {
            required_text(id, "resultId")?;
            let cached = SEARCH_RESULTS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(id)
                .cloned()
                .ok_or_else(|| {
                    FileToolError::new(
                        FileToolErrorCode::InvalidResultId,
                        "The result identifier is unknown or expired. Run the search again.",
                    )
                })?;
            approved_path(app, &cached, expect_file)
        }
        _ => Err(FileToolError::new(
            FileToolErrorCode::InvalidResultId,
            "Provide exactly one of 'path' or 'resultId'.",
        )),
    }
}

fn modified_millis(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn normalize_entity_text(value: &str) -> String {
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| match token {
            "one" => "1",
            "two" => "2",
            "three" => "3",
            "four" => "4",
            "five" => "5",
            "six" => "6",
            "seven" => "7",
            "eight" => "8",
            "nine" => "9",
            _ => token,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn search(
    app: &AppHandle,
    arguments: &SearchArguments,
    folders: bool,
) -> Result<Value, FileToolError> {
    required_text(&arguments.query, "query")?;
    if arguments
        .modified_within_days
        .is_some_and(|days| days == 0 || days > 3650)
    {
        return Err(FileToolError::new(
            FileToolErrorCode::SearchFailed,
            "'modifiedWithinDays' must be between 1 and 3650.",
        ));
    }

    let roots = if let Some(root) = &arguments.root {
        vec![resolve_root(app, root)?]
    } else {
        approved_roots(app)?
    };
    if roots.is_empty() {
        return Err(FileToolError::new(
            FileToolErrorCode::UnknownRoot,
            "No approved roots are currently available.",
        ));
    }

    let query = normalize_entity_text(arguments.query.trim());
    let tokens: Vec<_> = query.split_whitespace().collect();
    let extension = arguments
        .extension
        .as_deref()
        .map(|value| value.trim().trim_start_matches('.').to_lowercase())
        .filter(|value| !value.is_empty());
    let cutoff = arguments.modified_within_days.map(|days| {
        SystemTime::now()
            .checked_sub(Duration::from_secs(days as u64 * 86_400))
            .unwrap_or(UNIX_EPOCH)
    });

    let mut candidates = Vec::new();
    let mut visited = 0usize;
    let mut truncated = false;
    for root in &roots {
        let mut queue = VecDeque::from([(root.path.clone(), 0usize)]);
        while let Some((directory, depth)) = queue.pop_front() {
            let entries = fs::read_dir(&directory).map_err(|error| {
                FileToolError::new(
                    FileToolErrorCode::SearchFailed,
                    format!("Could not search '{}': {error}", directory.display()),
                )
            })?;
            for entry in entries.flatten() {
                visited += 1;
                if visited > MAX_VISITED {
                    truncated = true;
                    break;
                }
                let file_type = match entry.file_type() {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                if file_type.is_symlink() {
                    continue;
                }
                let is_match_kind = if folders {
                    file_type.is_dir()
                } else {
                    file_type.is_file()
                };
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let lower_name = normalize_entity_text(&name);
                let stem = normalize_entity_text(
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or(""),
                );
                let name_matches = if arguments.exact.unwrap_or(false) {
                    lower_name == query || stem == query
                } else {
                    tokens.iter().all(|token| lower_name.contains(token))
                };
                let extension_matches = extension.as_ref().is_none_or(|expected| {
                    path.extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                });
                let metadata = entry.metadata().ok();
                let recent_matches = cutoff.is_none_or(|cutoff| {
                    metadata
                        .as_ref()
                        .and_then(|value| value.modified().ok())
                        .is_some_and(|modified| modified >= cutoff)
                });

                if is_match_kind && name_matches && extension_matches && recent_matches {
                    let canonical = match path.canonicalize() {
                        Ok(value) if value.starts_with(&root.path) => value,
                        _ => continue,
                    };
                    let kind = if folders { "folder" } else { "file" };
                    let relevance = if lower_name == query {
                        120
                    } else if stem == query {
                        110
                    } else if stem.starts_with(&query) {
                        85
                    } else if tokens.iter().all(|token| stem.contains(token)) {
                        70
                    } else {
                        50
                    } + if root.kind == "project" { 5 } else { 0 };
                    candidates.push(SearchCandidate {
                        result_id: cache_candidate(&canonical, kind),
                        name,
                        path: display_path(&canonical),
                        kind,
                        extension: canonical
                            .extension()
                            .and_then(|value| value.to_str())
                            .map(str::to_lowercase),
                        modified_at: metadata.as_ref().and_then(modified_millis),
                        relevance,
                    });
                    if candidates.len() >= MAX_RESULTS {
                        truncated = true;
                        break;
                    }
                }
                if file_type.is_dir() && depth < MAX_DEPTH {
                    queue.push_back((path, depth + 1));
                }
            }
            if truncated {
                break;
            }
        }
        if truncated {
            break;
        }
    }

    candidates.sort_by(|left, right| {
        right
            .relevance
            .cmp(&left.relevance)
            .then_with(|| right.modified_at.cmp(&left.modified_at))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let count = candidates.len();
    Ok(json!({
        "candidates": candidates,
        "count": count,
        "ambiguous": count > 1,
        "truncated": truncated,
        "visited": visited,
        "approvedRoots": roots.iter().map(|root| json!({
            "id": root.id,
            "name": root.name,
            "kind": root.kind
        })).collect::<Vec<_>>(),
        "message": match count {
            0 => "No matching items were found.".to_string(),
            1 => "Found one matching item.".to_string(),
            value => format!("Found {value} matching items. Select a result identifier before opening."),
        }
    }))
}

#[cfg(windows)]
fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn shell_open(path: &Path) -> Result<(), FileToolError> {
    let operation = wide_null(OsStr::new("open"));
    let target = wide_null(path.as_os_str());
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(FileToolError::new(
            FileToolErrorCode::OpenFailed,
            format!("Windows could not open '{}'.", path.display()),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn shell_open(_path: &Path) -> Result<(), FileToolError> {
    Err(FileToolError::new(
        FileToolErrorCode::UnsupportedPlatform,
        "File tools are currently available only on Windows.",
    ))
}

pub fn open_folder(app: &AppHandle, folder: &str) -> Result<Value, FileToolError> {
    required_text(folder, "folder")?;
    let path = match resolve_root(app, folder) {
        Ok(root) => root.path,
        Err(_) => approved_path(app, Path::new(folder), false)?,
    };
    shell_open(&path)?;
    Ok(json!({
        "path": display_path(&path),
        "message": format!("Opened {}.", display_path(&path))
    }))
}

pub fn find_folders(app: &AppHandle, arguments: &SearchArguments) -> Result<Value, FileToolError> {
    search(app, arguments, true)
}

pub fn find_files(app: &AppHandle, arguments: &SearchArguments) -> Result<Value, FileToolError> {
    search(app, arguments, false)
}

pub fn open_file(app: &AppHandle, arguments: &FileTargetArguments) -> Result<Value, FileToolError> {
    let path = resolve_target(app, arguments, true)?;
    shell_open(&path)?;
    Ok(json!({
        "path": display_path(&path),
        "message": format!("Opened {}.", display_path(&path))
    }))
}

pub fn reveal_file(
    app: &AppHandle,
    arguments: &FileTargetArguments,
) -> Result<Value, FileToolError> {
    let path = resolve_target(app, arguments, true)?;
    let explorer = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("explorer.exe"))
        .filter(|candidate| candidate.is_file())
        .ok_or_else(|| {
            FileToolError::new(
                FileToolErrorCode::RevealFailed,
                "Windows File Explorer could not be located.",
            )
        })?;
    Command::new(explorer)
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map_err(|error| {
            FileToolError::new(
                FileToolErrorCode::RevealFailed,
                format!("Could not reveal '{}': {error}", path.display()),
            )
        })?;
    Ok(json!({
        "path": display_path(&path),
        "message": format!("Revealed {} in File Explorer.", display_path(&path))
    }))
}

pub fn list_projects(app: &AppHandle) -> Result<Vec<ProjectRecord>, String> {
    projects(app).map_err(|error| error.message)
}

pub fn add_project(
    app: &AppHandle,
    name: String,
    path: String,
    editor: String,
) -> Result<Vec<ProjectRecord>, String> {
    required_text(&name, "name").map_err(|error| error.message)?;
    required_text(&path, "path").map_err(|error| error.message)?;
    required_text(&editor, "editor").map_err(|error| error.message)?;
    let canonical = PathBuf::from(&path)
        .canonicalize()
        .map_err(|_| "The project folder does not exist.".to_string())?;
    if !canonical.is_dir() {
        return Err("The project path must be a folder.".into());
    }

    let mut records = projects(app).map_err(|error| error.message)?;
    if records.iter().any(|record| {
        record.name.eq_ignore_ascii_case(name.trim()) || PathBuf::from(&record.path) == canonical
    }) {
        return Err("A project with that name or path is already registered.".into());
    }
    records.push(ProjectRecord {
        voice_enabled: false,
        frontend_command: String::new(), backend_command: String::new(), working_directory: String::new(), development_url: String::new(), notes: String::new(),
        id: project_id(&canonical),
        name: name.trim().to_string(),
        path: display_path(&canonical),
        editor: editor.trim().to_string(),
    });
    records.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    let store = app
        .store(PROJECTS_FILE)
        .map_err(|error| format!("Could not open project settings: {error}"))?;
    store.set(
        PROJECTS_KEY,
        serde_json::to_value(&records)
            .map_err(|error| format!("Could not encode projects: {error}"))?,
    );
    store
        .save()
        .map_err(|error| format!("Could not save projects: {error}"))?;
    Ok(records)
}

#[tauri::command]
pub fn get_known_projects(app: AppHandle) -> Result<Vec<ProjectRecord>, String> {
    list_projects(&app)
}

#[tauri::command]
pub fn add_known_project(
    app: AppHandle,
    name: String,
    path: String,
    editor: String,
    confirmed: bool,
) -> Result<Vec<ProjectRecord>, String> {
    if !confirmed {
        return Err("Explicit user confirmation is required to approve a project root.".into());
    }
    add_project(&app, name, path, editor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_result_identifiers_are_path_specific() {
        let first = result_id(Path::new(r"C:\Users\Test\Downloads\report.pdf"), "file");
        let again = result_id(Path::new(r"C:\Users\Test\Downloads\report.pdf"), "file");
        let other = result_id(Path::new(r"C:\Users\Test\Downloads\other.pdf"), "file");
        assert_eq!(first, again);
        assert_ne!(first, other);
    }

    #[test]
    fn spoken_number_words_match_numeric_file_entities() {
        assert_eq!(
            normalize_entity_text("Chapter One.docx"),
            normalize_entity_text("Chapter 1.docx")
        );
        assert_ne!(
            normalize_entity_text("OneDrive"),
            normalize_entity_text("One Drive")
        );
    }

    #[test]
    fn search_arguments_reject_unknown_fields() {
        let result = serde_json::from_value::<SearchArguments>(json!({
            "query": "report",
            "drive": "C:"
        }));
        assert!(result.is_err());
    }

    #[test]
    fn target_requires_path_or_result_identifier() {
        let arguments = FileTargetArguments {
            path: None,
            result_id: None,
        };
        assert!(matches!(
            (&arguments.path, &arguments.result_id),
            (None, None)
        ));
    }
}

pub fn clear_cached_results() { if let Some(cache) = SEARCH_RESULTS.get() { if let Ok(mut cache) = cache.lock() { cache.clear(); } } }
