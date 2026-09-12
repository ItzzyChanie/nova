use crate::{
    application_control,
    file_control::{self, ProjectRecord},
    project_process::{self, ProjectProcess},
    settings,
    tool_router::{self, ToolResultStatus},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::local_store::SafeStoreExt;

pub static CONFIG_LOCK: Mutex<()> = Mutex::new(());
#[derive(Default)]
pub struct DeveloperService {
    running: Mutex<HashMap<String, Vec<ProjectProcess>>>,
    workflows: Mutex<HashSet<String>>,
}
impl DeveloperService {
    pub fn stop_all(&self) {
        if let Ok(mut runs) = self.running.lock() {
            runs.clear();
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workflow {
    #[serde(default)]
    pub voice_enabled: bool,
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub steps: Vec<Value>,
}

fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn save<T: Serialize>(app: &AppHandle, file: &str, key: &str, value: &T) -> Result<(), String> {
    let store = app.store(file).map_err(|e| e.to_string())?;
    let previous = store.get(key);
    store.set(key, serde_json::to_value(value).map_err(|e| e.to_string())?);
    if let Err(error) = store.save() {
        match previous {
            Some(value) => store.set(key, value),
            None => {
                store.delete(key);
            }
        }
        return Err(error.to_string());
    }
    Ok(())
}
fn dashboard(window: &WebviewWindow, confirmed: bool) -> Result<(), String> {
    if window.label() != "main" || !confirmed {
        return Err("Confirm this configuration change in the dashboard.".into());
    }
    Ok(())
}

pub fn local_url(value: &str) -> Result<url::Url, String> {
    let url = web_url(value)?;
    if !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")) {
        return Err("Development URLs must use localhost, 127.0.0.1 or [::1].".into());
    }
    Ok(url)
}
pub fn web_url(value: &str) -> Result<url::Url, String> {
    if !text(value, 2048) {
        return Err("Enter a valid HTTP(S) URL.".into());
    }
    let url = url::Url::parse(value).map_err(|_| "Enter an absolute HTTP(S) URL.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Only HTTP(S) URLs without embedded credentials are allowed.".into());
    }
    Ok(url)
}
pub fn open_url(app: &AppHandle, value: &str) -> Result<Value, String> {
    let url = web_url(value)?;
    if local_url(value).is_err() && !settings::network_access(app)? {
        return Err("External website access is OFF in Privacy & data.".into());
    }
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        let wide = |s: &str| {
            std::ffi::OsStr::new(s)
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let verb = wide("open");
        let target = wide(url.as_str());
        let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        );
        if result as isize <= 32 {
            return Err("Windows could not open the configured website.".into());
        }
        Ok(json!({"message":"Opened the configured website.","url":url.as_str()}))
    }
    #[cfg(not(windows))]
    {
        Err("Website opening requires Windows.".into())
    }
}
fn paths(project: &ProjectRecord) -> Result<(PathBuf, PathBuf), String> {
    if !Path::new(&project.path).is_absolute() {
        return Err("The project path must be absolute.".into());
    }
    let root = Path::new(&project.path)
        .canonicalize()
        .map_err(|_| "The project moved or was deleted. Edit its saved path.")?;
    if !root.is_dir() {
        return Err("The project path must be a directory.".into());
    }
    let cwd = if project.working_directory.trim().is_empty() {
        root.clone()
    } else {
        root.join(&project.working_directory)
            .canonicalize()
            .map_err(|_| "The working directory does not exist.")?
    };
    if !cwd.is_dir() || !cwd.starts_with(&root) {
        return Err("Working directory must be inside the project directory.".into());
    }
    Ok((root, cwd))
}
fn validate_project(project: &ProjectRecord) -> Result<(), String> {
    if !text(&project.id, 100) || !text(&project.name, 80) || project.notes.len() > 2000 {
        return Err("Project ID/name is invalid or notes are too long.".into());
    }
    if !matches!(
        project.editor.as_str(),
        "Visual Studio Code" | "Cursor" | "Visual Studio 2026" | "Android Studio"
    ) {
        return Err("Select a supported editor.".into());
    }
    paths(project)?;
    project_process::command_spec(&project.frontend_command)?;
    project_process::command_spec(&project.backend_command)?;
    if !project.development_url.is_empty() {
        local_url(&project.development_url)?;
    }
    Ok(())
}
#[tauri::command]
pub fn save_project_profile(
    window: WebviewWindow,
    mut project: ProjectRecord,
    confirmed: bool,
) -> Result<Vec<ProjectRecord>, String> {
    dashboard(&window, confirmed)?;
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| "Project settings are busy.")?;
    project.name = project.name.trim().to_string();
    validate_project(&project)?;
    let app = window.app_handle();
    if app
        .state::<DeveloperService>()
        .running
        .lock()
        .map_err(|_| "Projects are busy.")?
        .contains_key(&project.id)
    {
        return Err("Stop the project before changing its profile.".into());
    }
    let mut records = file_control::list_projects(app)?;
    if records
        .iter()
        .any(|p| p.id != project.id && p.name.eq_ignore_ascii_case(project.name.trim()))
    {
        return Err("A project with that name already exists.".into());
    }
    records.retain(|p| p.id != project.id);
    records.push(project);
    if records.len() > 100 {
        return Err("At most 100 project profiles are supported.".into());
    }
    save(app, "nova-projects.json", "projects", &records)?;
    Ok(records)
}
#[tauri::command]
pub fn delete_project_profile(
    window: WebviewWindow,
    id: String,
    confirmed: bool,
) -> Result<Vec<ProjectRecord>, String> {
    dashboard(&window, confirmed)?;
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| "Project settings are busy.")?;
    let app = window.app_handle();
    if app
        .state::<DeveloperService>()
        .running
        .lock()
        .map_err(|_| "Projects are busy.")?
        .contains_key(&id)
    {
        return Err("Stop the project before deleting its profile.".into());
    }
    let mut records = file_control::list_projects(app)?;
    records.retain(|p| p.id != id);
    save(app, "nova-projects.json", "projects", &records)?;
    Ok(records)
}
#[tauri::command]
pub fn get_project_status(app: AppHandle) -> Result<Value, String> {
    let service = app.state::<DeveloperService>();
    let runs = service.running.lock().map_err(|_| "Projects are busy.")?;
    Ok(Value::Object(
        runs.iter()
            .map(|(id, processes)| {
                (
                    id.clone(),
                    json!(if processes.iter().all(|p| p.running()) {
                        "running"
                    } else {
                        "exited"
                    }),
                )
            })
            .collect(),
    ))
}
fn find_project(app: &AppHandle, name: &str) -> Result<ProjectRecord, String> {
    file_control::list_projects(app)?
        .into_iter()
        .find(|p| p.id == name || p.name.eq_ignore_ascii_case(name.trim()))
        .ok_or("No saved project matches that name.".into())
}
pub fn project_action(app: &AppHandle, action: &str, name: &str) -> Result<Value, String> {
    let project = find_project(app, name)?;
    let service = app.state::<DeveloperService>();
    if action == "developer.stop_project" {
        let stopped = service
            .running
            .lock()
            .map_err(|_| "Projects are busy.")?
            .remove(&project.id)
            .is_some();
        return Ok(
            json!({"message":format!("{} {}.", project.name, if stopped {"stopped"} else {"has no processes owned by NOVA"})}),
        );
    }
    let (root, cwd) = paths(&project)?;
    if action == "developer.open_dev_url" {
        return open_url(app, &project.development_url);
    }
    if matches!(action, "developer.open_project" | "developer.open_editor") {
        application_control::open_project_editor(&project.editor, &root)?;
        return Ok(json!({"message":format!("Opened {} in {}.", project.name, project.editor)}));
    }
    if action != "developer.start_project" {
        return Err("Unsupported project action.".into());
    }
    {
        let mut runs = service.running.lock().map_err(|_| "Projects are busy.")?;
        if runs
            .get(&project.id)
            .is_some_and(|jobs| jobs.iter().any(|p| p.running()))
        {
            return Ok(json!({"message":format!("{} is already running.",project.name)}));
        }
        runs.remove(&project.id);
        let specifications: Vec<_> = [&project.frontend_command, &project.backend_command]
            .into_iter()
            .map(|c| project_process::resolve(c, &cwd))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        if specifications.is_empty() {
            return Err(
                "Configure a frontend or backend command before starting this project.".into(),
            );
        }
        if !project.development_url.is_empty() {
            let url = local_url(&project.development_url)?;
            let port = url
                .port_or_known_default()
                .ok_or("Development URL has no port.")?;
            let address = if url.host_str() == Some("[::1]") {
                std::net::SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port))
            } else {
                std::net::SocketAddr::from(([127, 0, 0, 1], port))
            };
            if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok() {
                return Err("The development port is already in use by another process. Use Open URL or free the port before starting.".into());
            }
        }
        application_control::open_project_editor(&project.editor, &root)?;
        let mut processes = Vec::new();
        for (program, args) in specifications {
            processes.push(ProjectProcess::start(&program, &args, &cwd)?);
        }
        runs.insert(project.id.clone(), processes);
    }
    if project.development_url.is_empty() {
        return Ok(
            json!({"message":format!("Started {}. No development URL is configured.",project.name)}),
        );
    }
    let url = local_url(&project.development_url)?;
    let port = url
        .port_or_known_default()
        .ok_or("Development URL has no port.")?;
    let address = if url.host_str() == Some("[::1]") {
        std::net::SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port))
    } else {
        std::net::SocketAddr::from(([127, 0, 0, 1], port))
    };
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        if !settings::nova_enabled(app)? {
            service
                .running
                .lock()
                .map_err(|_| "Projects are busy.")?
                .remove(&project.id);
            return Err("NOVA was turned off while starting the project.".into());
        }
        {
            let runs = service.running.lock().map_err(|_| "Projects are busy.")?;
            if !runs
                .get(&project.id)
                .is_some_and(|jobs| jobs.iter().all(|p| p.running()))
            {
                return Err("A project process exited or was stopped before the URL became ready. Stop the project and check its configured commands.".into());
            }
        }
        if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok() {
            open_url(app, url.as_str())?;
            return Ok(
                json!({"message":format!("Started {} and opened its development URL.",project.name)}),
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(format!("{} started, but its development URL was not ready within 20 seconds. Processes remain managed; use Stop or retry Open URL.",project.name))
}

#[tauri::command]
pub fn get_workflows(app: AppHandle) -> Result<Vec<Workflow>, String> {
    workflows(&app)
}
pub fn workflows(app: &AppHandle) -> Result<Vec<Workflow>, String> {
    let store = app
        .store("nova-workflows.json")
        .map_err(|e| e.to_string())?;
    match store.get("workflows") {
        None => Ok(vec![]),
        Some(v) => serde_json::from_value(v).map_err(|e| format!("Workflow data is invalid: {e}")),
    }
}
pub fn validate_workflow(workflow: &Workflow) -> Result<(), String> {
    if !text(&workflow.id, 100)
        || !text(&workflow.name, 80)
        || workflow.steps.is_empty()
        || workflow.steps.len() > 20
    {
        return Err("Workflow needs a name and 1-20 steps.".into());
    }
    for step in &workflow.steps {
        let tool = step
            .get("tool")
            .and_then(Value::as_str)
            .ok_or("Each step needs a tool.")?;
        if !matches!(
            tool,
            "application.open"
                | "application.focus"
                | "folder.open"
                | "file.open"
                | "system.set_volume"
                | "system.mute"
                | "system.unmute"
                | "window.focus"
                | "developer.open_project"
                | "developer.start_project"
                | "developer.stop_project"
                | "developer.open_editor"
                | "developer.open_dev_url"
                | "browser.open_url"
        ) {
            return Err(format!("{tool} is not an approved workflow step."));
        }
        tool_router::validate_model_request(step.clone()).map_err(|r| {
            r.error
                .map(|e| e.message)
                .unwrap_or("Invalid workflow step.".into())
        })?;
    }
    Ok(())
}
#[tauri::command]
pub fn save_workflow(
    window: WebviewWindow,
    mut workflow: Workflow,
    confirmed: bool,
) -> Result<Vec<Workflow>, String> {
    dashboard(&window, confirmed)?;
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| "Workflow settings are busy.")?;
    workflow.name = workflow.name.trim().to_string();
    validate_workflow(&workflow)?;
    let app = window.app_handle();
    let mut records = workflows(app)?;
    if app
        .state::<DeveloperService>()
        .workflows
        .lock()
        .map_err(|_| "Workflows are busy.")?
        .contains(&workflow.id)
    {
        return Err("Wait for the workflow to finish before editing it.".into());
    }
    if records
        .iter()
        .any(|w| w.id != workflow.id && w.name.eq_ignore_ascii_case(workflow.name.trim()))
    {
        return Err("A workflow with that name already exists.".into());
    }
    records.retain(|w| w.id != workflow.id);
    records.push(workflow);
    if records.len() > 100 {
        return Err("At most 100 workflows are supported.".into());
    }
    save(app, "nova-workflows.json", "workflows", &records)?;
    Ok(records)
}
#[tauri::command]
pub fn delete_workflow(
    window: WebviewWindow,
    id: String,
    confirmed: bool,
) -> Result<Vec<Workflow>, String> {
    dashboard(&window, confirmed)?;
    let _guard = CONFIG_LOCK
        .lock()
        .map_err(|_| "Workflow settings are busy.")?;
    let app = window.app_handle();
    if app
        .state::<DeveloperService>()
        .workflows
        .lock()
        .map_err(|_| "Workflows are busy.")?
        .contains(&id)
    {
        return Err("Wait for the workflow to finish before deleting it.".into());
    }
    let mut records = workflows(app)?;
    records.retain(|w| w.id != id);
    save(app, "nova-workflows.json", "workflows", &records)?;
    Ok(records)
}
pub fn run_workflow(app: &AppHandle, name: &str) -> Result<Value, String> {
    let _history = crate::command_history::SuppressHistory::new();
    let workflow = workflows(app)?
        .into_iter()
        .find(|w| w.id == name || w.name.eq_ignore_ascii_case(name.trim()))
        .ok_or("No saved workflow matches that name.")?;
    if !workflow.enabled {
        return Err("That workflow is disabled.".into());
    }
    validate_workflow(&workflow)?;
    let service = app.state::<DeveloperService>();
    if !service
        .workflows
        .lock()
        .map_err(|_| "Workflows are busy.")?
        .insert(workflow.id.clone())
    {
        return Err("That workflow is already running.".into());
    }
    struct Running<'a>(&'a DeveloperService, String);
    impl Drop for Running<'_> {
        fn drop(&mut self) {
            if let Ok(mut running) = self.0.workflows.lock() {
                running.remove(&self.1);
            }
        }
    }
    let _running = Running(&service, workflow.id.clone());
    let preferences = settings::skill_preferences(app)?;
    for (index, step) in workflow.steps.iter().enumerate() {
        let check =
            tool_router::preflight(step.clone(), &preferences, settings::nova_enabled(app)?);
        if check.status != ToolResultStatus::ConfirmationRequired {
            return Err(format!(
                "Step {} is not permitted. No steps were run.",
                index + 1
            ));
        }
        if step.get("tool").and_then(Value::as_str) == Some("browser.open_url") {
            let value = step["arguments"]["url"].as_str().ok_or("Missing URL.")?;
            if local_url(value).is_err() && !settings::network_access(app)? {
                return Err("External website access is OFF. No steps were run.".into());
            }
        }
    }
    for (index, step) in workflow.steps.iter().enumerate() {
        let result = tool_router::execute_workflow_step(app.clone(), step.clone());
        if result.status != ToolResultStatus::Completed {
            return Err(format!(
                "{} stopped at step {}: {}. Earlier completed steps were kept.",
                workflow.name,
                index + 1,
                result
                    .error
                    .map(|e| e.message)
                    .unwrap_or("Step failed".into())
            ));
        }
    }
    Ok(
        json!({"message":format!("{} completed {} steps.",workflow.name,workflow.steps.len()),"stepCount":workflow.steps.len()}),
    )
}

pub fn match_request(
    input: &str,
    projects: &[ProjectRecord],
    workflows: &[Workflow],
) -> Option<Value> {
    let value = input
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .to_lowercase();
    let value = value
        .strip_prefix("hey nova, ")
        .or_else(|| value.strip_prefix("hey nova "))
        .or_else(|| value.strip_prefix("nova, "))
        .or_else(|| value.strip_prefix("nova "))
        .unwrap_or(&value);
    let actions = [
        ("start ", "developer.start_project"),
        ("stop ", "developer.stop_project"),
        ("open ", "developer.open_project"),
    ];
    for (prefix, tool) in actions {
        if let Some(target) = value.strip_prefix(prefix) {
            let project_target = target
                .strip_suffix(" in vs code")
                .unwrap_or(target)
                .trim_start_matches("project ");
            let matches: Vec<_> = projects
                .iter()
                .filter(|p| {
                    p.name.eq_ignore_ascii_case(project_target)
                        || (project_target == "my project" && projects.len() == 1)
                })
                .collect();
            let flow: Vec<_> = workflows
                .iter()
                .filter(|w| w.name.eq_ignore_ascii_case(target) && prefix != "stop ")
                .collect();
            if matches.len() + flow.len() != 1 {
                return None;
            }
            if let Some(p) = matches.first() {
                return Some(json!({"tool":tool,"arguments":{"name":p.name}}));
            }
            if let Some(w) = flow.first() {
                return Some(json!({"tool":"workflow.run","arguments":{"name":w.name}}));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_reject_command_schemes_and_remote_development_hosts() {
        for value in [
            "file:///C:/x",
            "javascript:alert(1)",
            "https://user:pass@example.com",
            "https://localhost.evil.com",
        ] {
            assert!(local_url(value).is_err());
        }
        for value in [
            "http://localhost:5173",
            "http://127.0.0.1:3000",
            "http://[::1]:8000",
        ] {
            assert!(local_url(value).is_ok(), "{value}");
        }
    }
    #[test]
    fn workflow_rejects_recursion_shell_and_invalid_arguments() {
        for step in [
            json!({"tool":"workflow.run","arguments":{"name":"Loop"}}),
            json!({"tool":"developer.runScript","arguments":{"scriptId":"x"}}),
            json!({"tool":"system.set_volume","arguments":{"volume":255}}),
            json!({"tool":"developer.start_project","arguments":{"name":"X","command":"evil"}}),
        ] {
            assert!(validate_workflow(&Workflow {
                voice_enabled: false,
                id: "x".into(),
                name: "Test".into(),
                enabled: true,
                steps: vec![step]
            })
            .is_err());
        }
        assert!(validate_workflow(&Workflow {
            voice_enabled: false,
            id: "x".into(),
            name: "Coding Mode".into(),
            enabled: true,
            steps: vec![
                json!({"tool":"application.open","arguments":{"application":"Visual Studio Code"}}),
                json!({"tool":"system.set_volume","arguments":{"volume":25}})
            ]
        })
        .is_ok());
    }
}

pub fn voice_execution_approved(app: &AppHandle, request: &Value) -> bool {
    let Some(name) = request
        .get("arguments")
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    match request.get("tool").and_then(Value::as_str) {
        Some("workflow.run") => workflows(app).ok().is_some_and(|items| {
            items.iter().any(|w| {
                w.enabled && w.voice_enabled && (w.id == name || w.name.eq_ignore_ascii_case(name))
            })
        }),
        Some(
            "developer.open_project"
            | "developer.start_project"
            | "developer.stop_project"
            | "developer.open_editor"
            | "developer.open_dev_url",
        ) => find_project(app, name).is_ok_and(|p| p.voice_enabled),
        _ => false,
    }
}

#[cfg(test)]
mod integration_contract_tests {
    use super::*;
    fn profile() -> ProjectRecord {
        serde_json::from_value(json!({"id":"p","name":"ProctorX","path":"C:/Projects/ProctorX","editor":"Visual Studio Code"})).unwrap()
    }
    #[test]
    fn old_profiles_migrate_without_granting_commands_or_voice() {
        let p = profile();
        assert!(!p.voice_enabled);
        assert!(p.frontend_command.is_empty());
        assert!(p.backend_command.is_empty());
    }
    #[test]
    fn voice_uses_only_saved_names_and_never_accepts_shell_text() {
        let projects = vec![profile()];
        let flows = vec![Workflow {
            voice_enabled: false,
            id: "w".into(),
            name: "Coding Mode".into(),
            enabled: true,
            steps: vec![],
        }];
        for (input, tool) in [
            ("Hey NOVA, start ProctorX.", "developer.start_project"),
            ("Stop ProctorX.", "developer.stop_project"),
            ("Open my project in VS Code.", "developer.open_project"),
            ("start coding mode", "workflow.run"),
        ] {
            assert_eq!(
                match_request(input, &projects, &flows).unwrap()["tool"],
                tool
            );
        }
        assert!(match_request("start ProctorX && evil", &projects, &flows).is_none());
        assert!(match_request("start Unknown", &projects, &flows).is_none());
        assert!(match_request("Open my project", &[profile(), profile()], &flows).is_none());
    }
    #[test]
    fn project_working_directory_cannot_escape_root() {
        let root = std::env::temp_dir().join(format!("nova-path-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("project")).unwrap();
        let mut p = profile();
        p.path = root.join("project").to_string_lossy().into();
        p.working_directory = "..".into();
        assert!(paths(&p).is_err());
        p.working_directory.clear();
        assert!(paths(&p).is_ok());
        std::fs::remove_dir(root.join("project")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
