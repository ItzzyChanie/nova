use std::{env, ffi::OsString, path::PathBuf, process::Command};

use serde_json::{json, Value};

#[cfg(windows)]
use std::{mem::size_of, os::windows::ffi::OsStringExt, thread, time::Duration};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, SW_RESTORE, WM_CLOSE,
    },
};

#[cfg(windows)]
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
    RegKey,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationErrorCode {
    #[allow(dead_code)]
    UnsupportedPlatform,
    UnknownApplication,
    AmbiguousApplication,
    NotInstalled,
    NotRunning,
    LaunchFailed,
    FocusFailed,
    CloseFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationControlError {
    pub code: ApplicationErrorCode,
    pub message: String,
}

impl ApplicationControlError {
    fn new(code: ApplicationErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
struct KnownApplication {
    id: &'static str,
    display_name: &'static str,
    aliases: &'static [&'static str],
    executable_names: &'static [&'static str],
}

const APPLICATIONS: &[KnownApplication] = &[
    KnownApplication {
        id: "visual-studio-code",
        display_name: "Visual Studio Code",
        aliases: &[
            "visual studio code",
            "vs code",
            "vscode",
            "code",
            "code editor",
        ],
        executable_names: &["Code.exe"],
    },
    KnownApplication {
        id: "chrome",
        display_name: "Google Chrome",
        aliases: &["google chrome", "chrome", "chrome browser"],
        executable_names: &["chrome.exe"],
    },
    KnownApplication {
        id: "discord",
        display_name: "Discord",
        aliases: &["discord", "discord app"],
        executable_names: &["Discord.exe"],
    },
    KnownApplication {
        id: "spotify",
        display_name: "Spotify",
        aliases: &["spotify", "spotify music"],
        executable_names: &["Spotify.exe"],
    },
    KnownApplication {
        id: "notepad",
        display_name: "Notepad",
        aliases: &["notepad", "text editor", "windows notepad"],
        executable_names: &["Notepad.exe"],
    },
    KnownApplication {
        id: "file-explorer",
        display_name: "File Explorer",
        aliases: &["file explorer", "explorer", "windows explorer", "files"],
        executable_names: &["explorer.exe"],
    },
    KnownApplication {
        id: "docker-desktop",
        display_name: "Docker Desktop",
        aliases: &["docker desktop", "docker", "docker app"],
        executable_names: &["Docker Desktop.exe"],
    },
    KnownApplication {
        id: "microsoft-word",
        display_name: "Microsoft Word",
        aliases: &["microsoft word", "word", "ms word", "office word"],
        executable_names: &["WINWORD.EXE"],
    },
    KnownApplication {
        id: "visual-studio-2026",
        display_name: "Visual Studio 2026",
        aliases: &["visual studio 2026", "visual studio", "vs 2026", "vs2026"],
        executable_names: &["devenv.exe"],
    },
    KnownApplication {
        id: "capcut",
        display_name: "CapCut",
        aliases: &["capcut", "cap cut", "capcut editor", "video editor"],
        executable_names: &["CapCut.exe"],
    },
    KnownApplication {
        id: "powerpoint",
        display_name: "PowerPoint",
        aliases: &[
            "powerpoint",
            "power point",
            "microsoft powerpoint",
            "ms powerpoint",
        ],
        executable_names: &["POWERPNT.EXE"],
    },
    KnownApplication {
        id: "cursor",
        display_name: "Cursor",
        aliases: &["cursor", "cursor editor", "cursor ai", "cursor code editor"],
        executable_names: &["Cursor.exe"],
    },
    KnownApplication {
        id: "android-studio",
        display_name: "Android Studio",
        aliases: &["android studio", "android ide", "android editor"],
        executable_names: &["studio64.exe", "studio.exe"],
    },
    KnownApplication {
        id: "microsoft-edge",
        display_name: "Microsoft Edge",
        aliases: &["microsoft edge", "edge", "edge browser", "ms edge"],
        executable_names: &["msedge.exe"],
    },
];

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right_chars.iter().enumerate() {
            current.push(if left_char == *right_char {
                previous[right_index]
            } else {
                1 + previous[right_index]
                    .min(previous[right_index + 1])
                    .min(current[right_index])
            });
        }
        previous = current;
    }
    previous[right_chars.len()]
}

fn similarity(left: &str, right: &str) -> f32 {
    let length = left.chars().count().max(right.chars().count());
    if length == 0 {
        return 1.0;
    }
    1.0 - edit_distance(left, right) as f32 / length as f32
}

/// Resolves noisy STT output only when one registry application is a strong,
/// unambiguous match. The executable registry remains the source of truth.
pub fn known_application_names() -> Vec<&'static str> {
    APPLICATIONS
        .iter()
        .map(|application| application.display_name)
        .collect()
}

fn normalize_spoken_phrase(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn resolve_application_mention(utterance: &str) -> Option<&'static str> {
    let haystack = format!(" {} ", normalize_spoken_phrase(utterance));
    let mut matches = APPLICATIONS
        .iter()
        .filter_map(|application| {
            std::iter::once(application.display_name)
                .chain(application.aliases.iter().copied())
                .map(normalize_spoken_phrase)
                .filter(|alias| !alias.is_empty() && haystack.contains(&format!(" {alias} ")))
                .map(|alias| alias.len())
                .max()
                .map(|length| (application.display_name, length))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.1.cmp(&left.1));
    let (name, length) = *matches.first()?;
    (matches.get(1).is_none_or(|runner_up| runner_up.1 < length)).then_some(name)
}

pub fn resolve_spoken_application_name(spoken_target: &str) -> Option<&'static str> {
    let mut words = spoken_target
        .split_whitespace()
        .filter(|word| {
            !matches!(
                *word,
                "a" | "an"
                    | "the"
                    | "my"
                    | "app"
                    | "application"
                    | "please"
                    | "now"
                    | "for"
                    | "me"
                    | "to"
            )
        })
        .collect::<Vec<_>>();
    if words.len() >= 2
        && words.len() % 2 == 0
        && words[..words.len() / 2] == words[words.len() / 2..]
    {
        words.truncate(words.len() / 2);
    }
    let candidate = normalize(&words.join(" "));
    let mut scores = APPLICATIONS
        .iter()
        .map(|application| {
            let (best, best_alias_length) = std::iter::once(application.display_name)
                .chain(application.aliases.iter().copied())
                .map(|alias| {
                    let normalized_alias = normalize(alias);
                    (
                        similarity(&candidate, &normalized_alias),
                        normalized_alias.len(),
                    )
                })
                .max_by(|left, right| left.0.total_cmp(&right.0))
                .unwrap_or_default();
            (application.display_name, best, best_alias_length)
        })
        .collect::<Vec<_>>();
    scores.sort_by(|left, right| right.1.total_cmp(&left.1));
    let (name, best, alias_length) = *scores.first()?;
    let runner_up = scores.get(1).map_or(0.0, |entry| entry.1);
    let minimum = match alias_length {
        0..=4 => 1.0,
        5..=6 => 0.80,
        _ => 0.58,
    };
    (best >= minimum && best - runner_up >= 0.12).then_some(name)
}
fn resolve_known_application(
    requested: &str,
) -> Result<&'static KnownApplication, ApplicationControlError> {
    let normalized = normalize(requested);
    let matches: Vec<_> = APPLICATIONS
        .iter()
        .filter(|application| {
            normalize(application.display_name) == normalized
                || application
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == normalized)
        })
        .collect();

    match matches.as_slice() {
        [] => Err(ApplicationControlError::new(
            ApplicationErrorCode::UnknownApplication,
            format!(
                "'{requested}' is not in NOVA's known-application registry. Supported applications: {}.",
                APPLICATIONS
                    .iter()
                    .map(|application| application.display_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
        [application] => Ok(application),
        _ => Err(ApplicationControlError::new(
            ApplicationErrorCode::AmbiguousApplication,
            format!("'{requested}' matches more than one known application."),
        )),
    }
}

fn env_path(variable: &str, suffix: &str) -> Option<PathBuf> {
    env::var_os(variable).map(|base| PathBuf::from(base).join(suffix))
}

fn known_path_candidates(application: &KnownApplication) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    match application.id {
        "visual-studio-code" => {
            for variable in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(path) = env_path(variable, r"Programs\Microsoft VS Code\Code.exe") {
                    candidates.push(path);
                }
                if variable != "LOCALAPPDATA" {
                    if let Some(path) = env_path(variable, r"Microsoft VS Code\Code.exe") {
                        candidates.push(path);
                    }
                }
            }
        }
        "chrome" => {
            for variable in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(path) = env_path(variable, r"Google\Chrome\Application\chrome.exe") {
                    candidates.push(path);
                }
            }
        }
        "discord" => {
            if let Some(root) = env_path("LOCALAPPDATA", "Discord") {
                if let Ok(entries) = std::fs::read_dir(root) {
                    let mut versioned: Vec<_> = entries
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(|name| name.starts_with("app-"))
                        })
                        .map(|path| path.join("Discord.exe"))
                        .collect();
                    versioned.sort();
                    versioned.reverse();
                    candidates.extend(versioned);
                }
            }
        }
        "spotify" => {
            if let Some(path) = env_path("APPDATA", r"Spotify\Spotify.exe") {
                candidates.push(path);
            }
        }
        "notepad" => {
            if let Some(path) = env_path("SystemRoot", r"System32\Notepad.exe") {
                candidates.push(path);
            }
        }
        "file-explorer" => {
            if let Some(path) = env_path("SystemRoot", "explorer.exe") {
                candidates.push(path);
            }
        }
        "docker-desktop" => {
            for (variable, suffix) in [
                ("LOCALAPPDATA", r"Programs\DockerDesktop\Docker Desktop.exe"),
                ("LOCALAPPDATA", r"Programs\Docker\Docker\Docker Desktop.exe"),
                ("ProgramFiles", r"Docker\Docker\Docker Desktop.exe"),
                ("ProgramFiles", r"Docker\Docker\frontend\Docker Desktop.exe"),
            ] {
                if let Some(path) = env_path(variable, suffix) {
                    candidates.push(path);
                }
            }
        }
        "microsoft-word" => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                for suffix in [
                    r"Microsoft Office\root\Office16\WINWORD.EXE",
                    r"Microsoft Office\Office16\WINWORD.EXE",
                ] {
                    if let Some(path) = env_path(variable, suffix) {
                        candidates.push(path);
                    }
                }
            }
        }
        "visual-studio-2026" => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                for edition in ["Community", "Professional", "Enterprise", "Preview"] {
                    if let Some(path) = env_path(
                        variable,
                        &format!(r"Microsoft Visual Studio\18\{edition}\Common7\IDE\devenv.exe"),
                    ) {
                        candidates.push(path);
                    }
                }
            }
        }
        "capcut" => {
            for (variable, suffix) in [
                ("LOCALAPPDATA", r"CapCut\CapCut.exe"),
                ("LOCALAPPDATA", r"Programs\CapCut\CapCut.exe"),
                ("ProgramFiles", r"CapCut\CapCut.exe"),
                ("ProgramFiles(x86)", r"CapCut\CapCut.exe"),
            ] {
                if let Some(path) = env_path(variable, suffix) {
                    candidates.push(path);
                }
            }
        }
        "powerpoint" => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                for suffix in [
                    r"Microsoft Office\root\Office16\POWERPNT.EXE",
                    r"Microsoft Office\Office16\POWERPNT.EXE",
                ] {
                    if let Some(path) = env_path(variable, suffix) {
                        candidates.push(path);
                    }
                }
            }
        }
        "cursor" => {
            for (variable, suffix) in [
                ("LOCALAPPDATA", r"Programs\cursor\Cursor.exe"),
                ("LOCALAPPDATA", r"Programs\Cursor\Cursor.exe"),
                ("ProgramFiles", r"Cursor\Cursor.exe"),
            ] {
                if let Some(path) = env_path(variable, suffix) {
                    candidates.push(path);
                }
            }
        }
        "android-studio" => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                for product in ["Android Studio", "Android Studio Preview"] {
                    for executable in ["studio64.exe", "studio.exe"] {
                        if let Some(path) =
                            env_path(variable, &format!(r"Android\{product}\bin\{executable}"))
                        {
                            candidates.push(path);
                        }
                    }
                }
            }
        }
        "microsoft-edge" => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
                if let Some(path) = env_path(variable, r"Microsoft\Edge\Application\msedge.exe") {
                    candidates.push(path);
                }
            }
        }
        _ => {}
    }
    candidates
}

#[cfg(windows)]
fn app_paths_candidates(application: &KnownApplication) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for executable in application.executable_names {
        let subkey = format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{executable}");
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let root = RegKey::predef(hive);
            for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
                if let Ok(key) = root.open_subkey_with_flags(&subkey, KEY_READ | view) {
                    if let Ok(value) = key.get_value::<String, _>("") {
                        candidates.push(PathBuf::from(value.trim().trim_matches('"')));
                    }
                }
            }
        }
    }
    candidates
}

#[cfg(not(windows))]
fn app_paths_candidates(_application: &KnownApplication) -> Vec<PathBuf> {
    Vec::new()
}

fn path_candidates(application: &KnownApplication) -> Vec<PathBuf> {
    let mut candidates = known_path_candidates(application);
    candidates.extend(app_paths_candidates(application));

    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for executable in application.executable_names {
                candidates.push(directory.join(executable));
            }
        }
    }

    candidates
}

fn path_matches_application(application: &KnownApplication, path: &std::path::Path) -> bool {
    if application.id != "visual-studio-2026" {
        return true;
    }

    path.to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
        .contains(r"\microsoft visual studio\18\")
}

fn discover_executable(application: &KnownApplication) -> Result<PathBuf, ApplicationControlError> {
    path_candidates(application)
        .into_iter()
        .find(|path| path.is_file() && path_matches_application(application, path))
        .ok_or_else(|| {
            ApplicationControlError::new(
                ApplicationErrorCode::NotInstalled,
                format!(
                    "{} is known to NOVA but no installed executable was found.",
                    application.display_name
                ),
            )
        })
}

#[cfg(windows)]
fn wide_string(buffer: &[u16]) -> OsString {
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..length])
}

#[cfg(windows)]
fn process_executable_path(process_id: u32) -> Option<PathBuf> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 32_768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
        CloseHandle(process);

        (result != 0).then(|| PathBuf::from(OsString::from_wide(&buffer[..length as usize])))
    }
}

#[cfg(not(windows))]
fn process_executable_path(_process_id: u32) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn running_processes() -> Result<Vec<(u32, OsString)>, ApplicationControlError> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err(ApplicationControlError::new(
                ApplicationErrorCode::NotRunning,
                "Windows process enumeration is unavailable.",
            ));
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut processes = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                processes.push((entry.th32ProcessID, wide_string(&entry.szExeFile)));
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Ok(processes)
    }
}

#[cfg(not(windows))]
fn running_processes() -> Result<Vec<(u32, OsString)>, ApplicationControlError> {
    Err(ApplicationControlError::new(
        ApplicationErrorCode::UnsupportedPlatform,
        "Application controls are currently available only on Windows.",
    ))
}

fn matching_process_ids(
    application: &KnownApplication,
) -> Result<Vec<u32>, ApplicationControlError> {
    let names: Vec<String> = application
        .executable_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    Ok(running_processes()?
        .into_iter()
        .filter_map(|(process_id, executable)| {
            let executable = executable.to_string_lossy().to_ascii_lowercase();
            if !names.contains(&executable) {
                return None;
            }
            if application.id == "visual-studio-2026"
                && !process_executable_path(process_id)
                    .as_deref()
                    .is_some_and(|path| path_matches_application(application, path))
            {
                return None;
            }
            Some(process_id)
        })
        .collect())
}

#[cfg(windows)]
#[derive(Default)]
struct WindowSearch {
    process_ids: Vec<u32>,
    windows: Vec<HWND>,
}

#[cfg(windows)]
unsafe extern "system" fn collect_windows(window: HWND, parameter: LPARAM) -> i32 {
    let search = &mut *(parameter as *mut WindowSearch);
    let mut process_id = 0;
    GetWindowThreadProcessId(window, &mut process_id);
    let title_length = GetWindowTextLengthW(window);
    if search.process_ids.contains(&process_id) && IsWindowVisible(window) != 0 && title_length > 0
    {
        let mut title = vec![0u16; title_length as usize + 1];
        GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
        if wide_string(&title).to_string_lossy() != "Program Manager" {
            search.windows.push(window);
        }
    }
    1
}

#[cfg(windows)]
fn windows_for_processes(process_ids: Vec<u32>) -> Vec<HWND> {
    let mut search = WindowSearch {
        process_ids,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(
            Some(collect_windows),
            &mut search as *mut WindowSearch as LPARAM,
        );
    }
    search.windows
}

fn wait_for_windows(application: &KnownApplication) -> Result<Vec<HWND>, ApplicationControlError> {
    for attempt in 0..50 {
        let windows = windows_for_processes(matching_process_ids(application)?);
        if !windows.is_empty() || attempt == 49 {
            return Ok(windows);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(Vec::new())
}

pub fn open(application_name: &str) -> Result<Value, ApplicationControlError> {
    let application = resolve_known_application(application_name)?;
    let process_ids = matching_process_ids(application)?;
    let visible_windows = windows_for_processes(process_ids.clone());
    if !visible_windows.is_empty() {
        match focus_known(application) {
            Ok(_) => {
                return Ok(json!({
                    "application": application.display_name,
                    "applicationId": application.id,
                    "alreadyRunning": true,
                    "focused": true,
                    "message": format!("Focused the existing {} window.", application.display_name)
                }));
            }
            Err(error) => {
                eprintln!(
                    "Could not focus the existing {} window; requesting a new visible launch: {}",
                    application.display_name, error.message
                );
            }
        }
    }

    // Browsers and Explorer commonly keep background processes after their last
    // user-facing window closes. A process alone is not an open application for
    // this tool; launch a new visible window when none can be focused.
    let executable = discover_executable(application)?;
    Command::new(&executable).spawn().map_err(|error| {
        ApplicationControlError::new(
            ApplicationErrorCode::LaunchFailed,
            format!("Could not open {}: {error}", application.display_name),
        )
    })?;

    Ok(json!({
        "application": application.display_name,
        "applicationId": application.id,
        "alreadyRunning": false,
        "focused": false,
        "message": format!("Opened {}.", application.display_name)
    }))
}

#[cfg(windows)]
fn focus_known(application: &KnownApplication) -> Result<Value, ApplicationControlError> {
    let windows = wait_for_windows(application)?;
    let Some(window) = windows.first().copied() else {
        return Err(ApplicationControlError::new(
            ApplicationErrorCode::FocusFailed,
            format!(
                "{} is running but has no foreground-capable window.",
                application.display_name
            ),
        ));
    };

    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        if SetForegroundWindow(window) == 0 {
            return Err(ApplicationControlError::new(
                ApplicationErrorCode::FocusFailed,
                format!(
                    "Windows did not allow NOVA to focus {}.",
                    application.display_name
                ),
            ));
        }
    }

    Ok(json!({
        "application": application.display_name,
        "applicationId": application.id,
        "focused": true,
        "message": format!("Focused {}.", application.display_name)
    }))
}

#[cfg(not(windows))]
fn focus_known(_application: &KnownApplication) -> Result<Value, ApplicationControlError> {
    Err(ApplicationControlError::new(
        ApplicationErrorCode::UnsupportedPlatform,
        "Application focus is currently available only on Windows.",
    ))
}

pub fn focus(application_name: &str) -> Result<Value, ApplicationControlError> {
    let application = resolve_known_application(application_name)?;
    let process_ids = matching_process_ids(application)?;
    if process_ids.is_empty() {
        return Err(ApplicationControlError::new(
            ApplicationErrorCode::NotRunning,
            format!("{} is not running.", application.display_name),
        ));
    }
    focus_known(application)
}

pub fn is_running(application_name: &str) -> Result<Value, ApplicationControlError> {
    let application = resolve_known_application(application_name)?;
    let process_ids = matching_process_ids(application)?;
    Ok(json!({
        "application": application.display_name,
        "applicationId": application.id,
        "running": !process_ids.is_empty(),
        "processCount": process_ids.len(),
        "message": if process_ids.is_empty() {
            format!("{} is not running.", application.display_name)
        } else {
            format!("{} is running.", application.display_name)
        }
    }))
}

pub fn list_running() -> Result<Value, ApplicationControlError> {
    let mut applications = Vec::new();
    for application in APPLICATIONS {
        let process_ids = matching_process_ids(application)?;
        if !process_ids.is_empty() {
            applications.push(json!({
                "application": application.display_name,
                "applicationId": application.id,
                "processCount": process_ids.len()
            }));
        }
    }

    Ok(json!({
        "applications": applications,
        "count": applications.len(),
        "message": format!("Found {} known running applications.", applications.len())
    }))
}

#[cfg(windows)]
pub fn close(application_name: &str) -> Result<Value, ApplicationControlError> {
    let application = resolve_known_application(application_name)?;
    let process_ids = matching_process_ids(application)?;
    if process_ids.is_empty() {
        return Err(ApplicationControlError::new(
            ApplicationErrorCode::NotRunning,
            format!("{} is not running.", application.display_name),
        ));
    }

    let windows = wait_for_windows(application)?;
    if windows.is_empty() {
        return Err(ApplicationControlError::new(
            ApplicationErrorCode::CloseFailed,
            format!(
                "{} has no unambiguous top-level window NOVA can close safely.",
                application.display_name
            ),
        ));
    }

    let mut requested = 0usize;
    for window in windows {
        unsafe {
            if PostMessageW(window, WM_CLOSE, 0, 0) != 0 {
                requested += 1;
            }
        }
    }
    if requested == 0 {
        return Err(ApplicationControlError::new(
            ApplicationErrorCode::CloseFailed,
            format!(
                "Windows rejected the close request for {}.",
                application.display_name
            ),
        ));
    }

    Ok(json!({
        "application": application.display_name,
        "applicationId": application.id,
        "windowsRequested": requested,
        "message": format!("Requested {} to close.", application.display_name)
    }))
}

#[cfg(not(windows))]
pub fn close(_application_name: &str) -> Result<Value, ApplicationControlError> {
    Err(ApplicationControlError::new(
        ApplicationErrorCode::UnsupportedPlatform,
        "Application close is currently available only on Windows.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_supported_aliases_case_and_punctuation_insensitively() {
        assert_eq!(
            resolve_known_application("VS Code").unwrap().id,
            "visual-studio-code"
        );
        assert_eq!(
            resolve_known_application("code-editor").unwrap().id,
            "visual-studio-code"
        );
        assert_eq!(resolve_known_application("CHROME").unwrap().id, "chrome");
        assert_eq!(
            resolve_known_application("Windows Explorer").unwrap().id,
            "file-explorer"
        );
    }

    #[test]
    fn noisy_speech_application_names_resolve_only_when_unambiguous() {
        assert_eq!(
            resolve_spoken_application_name("notebagged"),
            Some("Notepad")
        );
        assert_eq!(
            resolve_spoken_application_name("note pad note pad"),
            Some("Notepad")
        );
        assert_eq!(
            resolve_spoken_application_name("visual studio coat"),
            Some("Visual Studio Code")
        );
        assert_eq!(resolve_spoken_application_name("world"), None);
        assert_eq!(resolve_spoken_application_name("something unknown"), None);
    }

    #[test]
    fn application_mentions_are_recovered_without_substring_guessing() {
        assert_eq!(
            resolve_application_mention("Microsoft Edge is still not open"),
            Some("Microsoft Edge")
        );
        assert_eq!(
            resolve_application_mention("Can you close Visual Studio Code now"),
            Some("Visual Studio Code")
        );
        assert_eq!(resolve_application_mention("open the document"), None);
    }
    #[test]
    fn rejects_unknown_applications() {
        let error = resolve_known_application("Definitely Not Installed").unwrap_err();
        assert_eq!(error.code, ApplicationErrorCode::UnknownApplication);
    }

    #[test]
    fn every_alias_resolves_to_only_its_owner() {
        for application in APPLICATIONS {
            for alias in application.aliases {
                assert_eq!(resolve_known_application(alias).unwrap().id, application.id);
            }
        }
    }

    #[test]
    fn developer_panel_applications_are_registered_with_voice_friendly_aliases() {
        for (request, expected_id) in [
            ("Docker", "docker-desktop"),
            ("Word", "microsoft-word"),
            ("VS 2026", "visual-studio-2026"),
            ("Cap Cut", "capcut"),
            ("Microsoft PowerPoint", "powerpoint"),
            ("Cursor AI", "cursor"),
            ("Android Studio", "android-studio"),
            ("Edge browser", "microsoft-edge"),
        ] {
            assert_eq!(resolve_known_application(request).unwrap().id, expected_id);
        }
    }

    #[test]
    fn visual_studio_2026_does_not_match_visual_studio_2022_paths() {
        let application = resolve_known_application("Visual Studio 2026").unwrap();
        assert!(path_matches_application(
            application,
            std::path::Path::new(
                r"C:\Program Files\Microsoft Visual Studio\18\Community\Common7\IDE\devenv.exe"
            )
        ));
        assert!(!path_matches_application(
            application,
            std::path::Path::new(
                r"C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\devenv.exe"
            )
        ));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires the Developer Tools applications to be installed locally"]
    fn developer_panel_applications_are_discoverable_on_windows() {
        for name in [
            "Docker Desktop",
            "Microsoft Word",
            "Visual Studio 2026",
            "CapCut",
            "PowerPoint",
            "Cursor",
            "Android Studio",
            "Microsoft Edge",
        ] {
            let application = resolve_known_application(name).unwrap();
            let executable = discover_executable(application)
                .unwrap_or_else(|error| panic!("{name}: {}", error.message));
            assert!(
                executable.is_file(),
                "{} does not exist",
                executable.display()
            );
        }
    }
}


pub fn open_project_editor(editor: &str, path: &std::path::Path) -> Result<(), String> {
    if !matches!(editor, "Visual Studio Code" | "Cursor" | "Visual Studio 2026" | "Android Studio") {
        return Err("Choose a supported project editor.".into());
    }
    let application = resolve_known_application(editor).map_err(|e| e.message)?;
    let executable = discover_executable(application).map_err(|e| e.message)?;
    let mut command = std::process::Command::new(executable);
    command.arg(path);
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| format!("Could not open the project editor: {e}"))?;
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}
