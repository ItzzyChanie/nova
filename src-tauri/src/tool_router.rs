use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::{
    application_control, command_history, file_control, settings, system_control, window_control,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolCategory {
    Application,
    File,
    Folder,
    System,
    Browser,
    Window,
    Developer,
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolRiskLevel {
    Safe,
    Sensitive,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolPermission {
    Allowed,
    Denied,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ToolName {
    #[serde(rename = "developer.open_project")]
    DeveloperOpenProject,
    #[serde(rename = "developer.start_project")]
    DeveloperStartProject,
    #[serde(rename = "developer.stop_project")]
    DeveloperStopProject,
    #[serde(rename = "developer.open_editor")]
    DeveloperOpenEditor,
    #[serde(rename = "developer.open_dev_url")]
    DeveloperOpenDevUrl,
    #[serde(rename = "workflow.run")]
    WorkflowRun,
    #[serde(rename = "browser.open_url")]
    BrowserOpenUrl,

    #[serde(rename = "application.open")]
    ApplicationOpen,
    #[serde(rename = "application.close")]
    ApplicationClose,
    #[serde(rename = "application.focus")]
    ApplicationFocus,
    #[serde(rename = "application.is_running")]
    ApplicationIsRunning,
    #[serde(rename = "application.list_running")]
    ApplicationListRunning,
    #[serde(rename = "folder.open")]
    FolderOpen,
    #[serde(rename = "folder.find")]
    FolderFind,
    #[serde(rename = "file.open")]
    FileOpen,
    #[serde(rename = "file.find_by_name")]
    FileFindByName,
    #[serde(rename = "file.reveal_in_explorer")]
    FileRevealInExplorer,
    #[serde(rename = "file.delete")]
    FileDelete,
    #[serde(rename = "folder.list")]
    FolderList,
    #[serde(rename = "system.get_volume")]
    SystemGetVolume,
    #[serde(rename = "system.set_volume")]
    SystemSetVolume,
    #[serde(rename = "system.adjust_volume")]
    SystemAdjustVolume,
    #[serde(rename = "system.mute")]
    SystemMute,
    #[serde(rename = "system.unmute")]
    SystemUnmute,
    #[serde(rename = "system.get_battery")]
    SystemGetBattery,
    #[serde(rename = "system.get_cpu_usage")]
    SystemGetCpuUsage,
    #[serde(rename = "system.get_memory_usage")]
    SystemGetMemoryUsage,
    #[serde(rename = "system.take_screenshot")]
    SystemTakeScreenshot,
    #[serde(rename = "system.shutdown")]
    SystemShutdown,
    #[serde(rename = "browser.search")]
    BrowserSearch,
    #[serde(rename = "window.list")]
    WindowList,
    #[serde(rename = "window.focus")]
    WindowFocus,
    #[serde(rename = "developer.runScript")]
    DeveloperRunScript,
    #[serde(rename = "workflow.preview")]
    WorkflowPreview,
}

impl FromStr for ToolName {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "developer.open_project" => Ok(Self::DeveloperOpenProject),
            "developer.start_project" => Ok(Self::DeveloperStartProject),
            "developer.stop_project" => Ok(Self::DeveloperStopProject),
            "developer.open_editor" => Ok(Self::DeveloperOpenEditor),
            "developer.open_dev_url" => Ok(Self::DeveloperOpenDevUrl),
            "workflow.run" => Ok(Self::WorkflowRun),
            "browser.open_url" => Ok(Self::BrowserOpenUrl),
            "application.open" => Ok(Self::ApplicationOpen),
            "application.close" => Ok(Self::ApplicationClose),
            "application.focus" => Ok(Self::ApplicationFocus),
            "application.is_running" => Ok(Self::ApplicationIsRunning),
            "application.list_running" => Ok(Self::ApplicationListRunning),
            "folder.open" => Ok(Self::FolderOpen),
            "folder.find" => Ok(Self::FolderFind),
            "file.open" => Ok(Self::FileOpen),
            "file.find_by_name" => Ok(Self::FileFindByName),
            "file.reveal_in_explorer" => Ok(Self::FileRevealInExplorer),
            "file.delete" => Ok(Self::FileDelete),
            "folder.list" => Ok(Self::FolderList),
            "system.get_volume" => Ok(Self::SystemGetVolume),
            "system.set_volume" => Ok(Self::SystemSetVolume),
            "system.adjust_volume" => Ok(Self::SystemAdjustVolume),
            "system.mute" => Ok(Self::SystemMute),
            "system.unmute" => Ok(Self::SystemUnmute),
            "system.get_battery" => Ok(Self::SystemGetBattery),
            "system.get_cpu_usage" => Ok(Self::SystemGetCpuUsage),
            "system.get_memory_usage" => Ok(Self::SystemGetMemoryUsage),
            "system.take_screenshot" => Ok(Self::SystemTakeScreenshot),
            "system.shutdown" => Ok(Self::SystemShutdown),
            "browser.search" => Ok(Self::BrowserSearch),
            "window.list" => Ok(Self::WindowList),
            "window.focus" => Ok(Self::WindowFocus),
            "developer.runScript" => Ok(Self::DeveloperRunScript),
            "workflow.preview" => Ok(Self::WorkflowPreview),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillId {
    Apps,
    System,
    Web,
    Calendar,
    Files,
    Drafting,
    Windows,
    Scripts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPreferences {
    pub apps: bool,
    pub system: bool,
    pub web: bool,
    pub calendar: bool,
    pub files: bool,
    pub drafting: bool,
    pub windows: bool,
    pub scripts: bool,
}

impl Default for SkillPreferences {
    fn default() -> Self {
        Self {
            apps: true,
            system: false,
            web: false,
            calendar: false,
            files: false,
            drafting: false,
            windows: true,
            scripts: false,
        }
    }
}

impl SkillPreferences {
    pub fn set(&mut self, skill: SkillId, enabled: bool) {
        match skill {
            SkillId::Apps => self.apps = enabled,
            SkillId::System => self.system = enabled,
            SkillId::Web => self.web = enabled,
            SkillId::Calendar => self.calendar = enabled,
            SkillId::Files => self.files = enabled,
            SkillId::Drafting => self.drafting = enabled,
            SkillId::Windows => self.windows = enabled,
            SkillId::Scripts => self.scripts = enabled,
        }
    }

    fn enabled(&self, skill: SkillId) -> bool {
        match skill {
            SkillId::Apps => self.apps,
            SkillId::System => self.system,
            SkillId::Web => self.web,
            SkillId::Calendar => self.calendar,
            SkillId::Files => self.files,
            SkillId::Drafting => self.drafting,
            SkillId::Windows => self.windows,
            SkillId::Scripts => self.scripts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolPolicy {
    category: ToolCategory,
    risk: ToolRiskLevel,
    permission: ToolPermission,
    required_skill: Option<SkillId>,
}

impl ToolName {
    fn policy(self) -> ToolPolicy {
        match self {
            Self::ApplicationOpen => ToolPolicy {
                category: ToolCategory::Application,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Apps),
            },
            Self::ApplicationClose => ToolPolicy {
                category: ToolCategory::Application,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Apps),
            },
            Self::ApplicationFocus => ToolPolicy {
                category: ToolCategory::Application,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Apps),
            },
            Self::ApplicationIsRunning | Self::ApplicationListRunning => ToolPolicy {
                category: ToolCategory::Application,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Allowed,
                required_skill: Some(SkillId::Apps),
            },
            Self::FolderOpen | Self::FolderFind => ToolPolicy {
                category: ToolCategory::Folder,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Files),
            },
            Self::FileOpen | Self::FileFindByName | Self::FileRevealInExplorer => ToolPolicy {
                category: ToolCategory::File,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Files),
            },
            Self::FileDelete => ToolPolicy {
                category: ToolCategory::File,
                risk: ToolRiskLevel::Destructive,
                permission: ToolPermission::Denied,
                required_skill: Some(SkillId::Files),
            },
            Self::FolderList => ToolPolicy {
                category: ToolCategory::Folder,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Files),
            },
            Self::SystemGetVolume
            | Self::SystemGetBattery
            | Self::SystemGetCpuUsage
            | Self::SystemGetMemoryUsage => ToolPolicy {
                category: ToolCategory::System,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Allowed,
                required_skill: Some(SkillId::System),
            },
            Self::SystemSetVolume
            | Self::SystemAdjustVolume
            | Self::SystemMute
            | Self::SystemUnmute => ToolPolicy {
                category: ToolCategory::System,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::System),
            },
            Self::SystemTakeScreenshot => ToolPolicy {
                category: ToolCategory::System,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::System),
            },
            Self::SystemShutdown => ToolPolicy {
                category: ToolCategory::System,
                risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Denied,
                required_skill: Some(SkillId::System),
            },
            Self::BrowserSearch => ToolPolicy {
                category: ToolCategory::Browser,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Web),
            },
            Self::WindowList => ToolPolicy {
                category: ToolCategory::Window,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Allowed,
                required_skill: Some(SkillId::Windows),
            },
            Self::WindowFocus => ToolPolicy {
                category: ToolCategory::Window,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Confirm,
                required_skill: Some(SkillId::Windows),
            },
            Self::DeveloperOpenProject | Self::DeveloperStartProject | Self::DeveloperStopProject | Self::DeveloperOpenEditor | Self::DeveloperOpenDevUrl => ToolPolicy {
                category: ToolCategory::Developer, risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm, required_skill: Some(SkillId::Scripts),
            },
            Self::WorkflowRun => ToolPolicy {
                category: ToolCategory::Workflow, risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm, required_skill: None,
            },
            Self::BrowserOpenUrl => ToolPolicy {
                category: ToolCategory::Browser, risk: ToolRiskLevel::Sensitive,
                permission: ToolPermission::Confirm, required_skill: Some(SkillId::Web),
            },
            Self::DeveloperRunScript => ToolPolicy {
                category: ToolCategory::Developer,
                risk: ToolRiskLevel::Destructive,
                permission: ToolPermission::Denied,
                required_skill: Some(SkillId::Scripts),
            },
            Self::WorkflowPreview => ToolPolicy {
                category: ToolCategory::Workflow,
                risk: ToolRiskLevel::Safe,
                permission: ToolPermission::Allowed,
                required_skill: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub request_id: Option<String>,
    pub tool: ToolName,
    pub arguments: ToolArguments,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ToolArguments {
    RegisteredTarget(RegisteredTargetArguments),
    BrowserOpenUrl(UrlArguments),
    ApplicationOpen(ApplicationTargetArguments),
    ApplicationClose(ApplicationTargetArguments),
    ApplicationFocus(ApplicationTargetArguments),
    ApplicationIsRunning(ApplicationTargetArguments),
    ApplicationListRunning(EmptyArguments),
    FolderOpen(FolderOpenArguments),
    FolderFind(file_control::SearchArguments),
    FileOpen(file_control::FileTargetArguments),
    FileFindByName(file_control::SearchArguments),
    FileRevealInExplorer(file_control::FileTargetArguments),
    FileDelete(PathArguments),
    FolderList(PathArguments),
    SystemGetVolume(EmptyArguments),
    SystemSetVolume(VolumeArguments),
    SystemAdjustVolume(VolumeAdjustmentArguments),
    SystemMute(EmptyArguments),
    SystemUnmute(EmptyArguments),
    SystemGetBattery(EmptyArguments),
    SystemGetCpuUsage(EmptyArguments),
    SystemGetMemoryUsage(EmptyArguments),
    SystemTakeScreenshot(EmptyArguments),
    SystemShutdown(EmptyArguments),
    BrowserSearch(BrowserSearchArguments),
    WindowList(EmptyArguments),
    WindowFocus(window_control::WindowFocusArguments),
    DeveloperRunScript(ScriptArguments),
    WorkflowPreview(WorkflowPreviewArguments),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisteredTargetArguments { pub name: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlArguments { pub url: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationTargetArguments {
    pub application: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderOpenArguments {
    pub folder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathArguments {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyArguments {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeArguments {
    pub volume: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeAdjustmentArguments {
    pub delta: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserSearchArguments {
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScriptArguments {
    pub script_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPreviewArguments {
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolResultStatus {
    Completed,
    Rejected,
    Denied,
    ConfirmationRequired,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolErrorCode {
    InvalidTool,
    InvalidArguments,
    PermissionDenied,
    ConfirmationRequired,
    NotImplemented,
    SettingsUnavailable,
    UnknownApplication,
    AmbiguousApplication,
    ApplicationNotInstalled,
    ApplicationNotRunning,
    ApplicationLaunchFailed,
    ApplicationFocusFailed,
    ApplicationCloseFailed,
    UnsupportedPlatform,
    UnknownRoot,
    PathOutsideApprovedRoots,
    FileNotFound,
    FolderNotFound,
    InvalidResultId,
    InvalidProject,
    FileSearchFailed,
    FileOpenFailed,
    FileRevealFailed,
    VolumeUnavailable,
    SystemInfoUnavailable,
    ScreenshotFailed,
    InvalidWindow,
    WindowNotFound,
    AmbiguousWindow,
    WindowFocusFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolError {
    pub code: ToolErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub request_id: Option<String>,
    pub tool: Option<ToolName>,
    pub category: Option<ToolCategory>,
    pub risk: Option<ToolRiskLevel>,
    pub permission: Option<ToolPermission>,
    pub status: ToolResultStatus,
    pub data: Option<Value>,
    pub error: Option<ToolError>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IncomingToolRequest {
    request_id: Option<String>,
    tool: String,
    arguments: Value,
}

fn error_result(
    request_id: Option<String>,
    tool: Option<ToolName>,
    policy: Option<ToolPolicy>,
    status: ToolResultStatus,
    code: ToolErrorCode,
    message: impl Into<String>,
) -> ToolResult {
    ToolResult {
        request_id,
        tool,
        category: policy.map(|value| value.category),
        risk: policy.map(|value| value.risk),
        permission: policy.map(|value| value.permission),
        status,
        data: None,
        error: Some(ToolError {
            code,
            message: message.into(),
        }),
    }
}

fn required_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("'{field}' must be a non-empty string."));
    }
    if value.len() > 512 {
        return Err(format!("'{field}' exceeds the 512-character limit."));
    }
    Ok(())
}

fn decode_arguments(tool: ToolName, value: Value) -> Result<ToolArguments, String> {
    let invalid = |error: serde_json::Error| format!("Invalid arguments for this tool: {error}");
    match tool {
        ToolName::ApplicationOpen => {
            let arguments: ApplicationTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.application, "application")?;
            Ok(ToolArguments::ApplicationOpen(arguments))
        }
        ToolName::ApplicationClose => {
            let arguments: ApplicationTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.application, "application")?;
            Ok(ToolArguments::ApplicationClose(arguments))
        }
        ToolName::ApplicationFocus => {
            let arguments: ApplicationTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.application, "application")?;
            Ok(ToolArguments::ApplicationFocus(arguments))
        }
        ToolName::ApplicationIsRunning => {
            let arguments: ApplicationTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.application, "application")?;
            Ok(ToolArguments::ApplicationIsRunning(arguments))
        }
        ToolName::ApplicationListRunning => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::ApplicationListRunning(arguments))
        }
        ToolName::FolderOpen => {
            let arguments: FolderOpenArguments = serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.folder, "folder")?;
            Ok(ToolArguments::FolderOpen(arguments))
        }
        ToolName::FolderFind => {
            let arguments: file_control::SearchArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.query, "query")?;
            Ok(ToolArguments::FolderFind(arguments))
        }
        ToolName::FileOpen => {
            let arguments: file_control::FileTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::FileOpen(arguments))
        }
        ToolName::FileFindByName => {
            let arguments: file_control::SearchArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.query, "query")?;
            Ok(ToolArguments::FileFindByName(arguments))
        }
        ToolName::FileRevealInExplorer => {
            let arguments: file_control::FileTargetArguments =
                serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::FileRevealInExplorer(arguments))
        }
        ToolName::FileDelete => {
            let arguments: PathArguments = serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.path, "path")?;
            Ok(ToolArguments::FileDelete(arguments))
        }
        ToolName::FolderList => {
            let arguments: PathArguments = serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.path, "path")?;
            Ok(ToolArguments::FolderList(arguments))
        }
        ToolName::SystemGetVolume => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemGetVolume(arguments))
        }
        ToolName::SystemSetVolume => {
            let arguments: VolumeArguments = serde_json::from_value(value).map_err(invalid)?;
            if arguments.volume > 100 {
                return Err("'volume' must be between 0 and 100.".into());
            }
            Ok(ToolArguments::SystemSetVolume(arguments))
        }
        ToolName::SystemAdjustVolume => {
            let arguments: VolumeAdjustmentArguments =
                serde_json::from_value(value).map_err(invalid)?;
            if arguments.delta == 0 || !(-100..=100).contains(&arguments.delta) {
                return Err("'delta' must be between -100 and 100 and cannot be zero.".into());
            }
            Ok(ToolArguments::SystemAdjustVolume(arguments))
        }
        ToolName::SystemMute => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemMute(arguments))
        }
        ToolName::SystemUnmute => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemUnmute(arguments))
        }
        ToolName::SystemGetBattery => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemGetBattery(arguments))
        }
        ToolName::SystemGetCpuUsage => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemGetCpuUsage(arguments))
        }
        ToolName::SystemGetMemoryUsage => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemGetMemoryUsage(arguments))
        }
        ToolName::SystemTakeScreenshot => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemTakeScreenshot(arguments))
        }
        ToolName::SystemShutdown => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::SystemShutdown(arguments))
        }
        ToolName::BrowserSearch => {
            let arguments: BrowserSearchArguments =
                serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.query, "query")?;
            Ok(ToolArguments::BrowserSearch(arguments))
        }
        ToolName::WindowList => {
            let arguments: EmptyArguments = serde_json::from_value(value).map_err(invalid)?;
            Ok(ToolArguments::WindowList(arguments))
        }
        ToolName::WindowFocus => {
            let arguments: window_control::WindowFocusArguments =
                serde_json::from_value(value).map_err(invalid)?;
            if arguments.window_id.is_none() {
                required_text(arguments.title.as_deref().unwrap_or(""), "title")?;
            }
            Ok(ToolArguments::WindowFocus(arguments))
        }
        ToolName::DeveloperRunScript => {
            let arguments: ScriptArguments = serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.script_id, "scriptId")?;
            Ok(ToolArguments::DeveloperRunScript(arguments))
        }
        ToolName::DeveloperOpenProject | ToolName::DeveloperStartProject | ToolName::DeveloperStopProject | ToolName::DeveloperOpenEditor | ToolName::DeveloperOpenDevUrl | ToolName::WorkflowRun => {
            let arguments: RegisteredTargetArguments = serde_json::from_value(value).map_err(invalid)?;
            required_text(&arguments.name, "name")?;
            Ok(ToolArguments::RegisteredTarget(arguments))
        }
        ToolName::BrowserOpenUrl => {
            let arguments: UrlArguments = serde_json::from_value(value).map_err(invalid)?;
            crate::developer::web_url(&arguments.url)?;
            Ok(ToolArguments::BrowserOpenUrl(arguments))
        }
        ToolName::WorkflowPreview => {
            let arguments: WorkflowPreviewArguments =
                serde_json::from_value(value).map_err(invalid)?;
            if arguments.steps.is_empty() {
                return Err("'steps' must contain at least one item.".into());
            }
            if arguments.steps.len() > 20 {
                return Err("'steps' cannot contain more than 20 items.".into());
            }
            for step in &arguments.steps {
                required_text(step, "steps[]")?;
            }
            Ok(ToolArguments::WorkflowPreview(arguments))
        }
    }
}

fn parse_request(value: Value) -> Result<ToolRequest, ToolResult> {
    let incoming: IncomingToolRequest = serde_json::from_value(value).map_err(|error| {
        error_result(
            None,
            None,
            None,
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidArguments,
            format!("Invalid tool request envelope: {error}"),
        )
    })?;

    if let Some(request_id) = &incoming.request_id {
        required_text(request_id, "requestId").map_err(|message| {
            error_result(
                incoming.request_id.clone(),
                None,
                None,
                ToolResultStatus::Rejected,
                ToolErrorCode::InvalidArguments,
                message,
            )
        })?;
    }

    let tool = ToolName::from_str(&incoming.tool).map_err(|_| {
        error_result(
            incoming.request_id.clone(),
            None,
            None,
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidTool,
            format!("Unknown tool '{}'.", incoming.tool),
        )
    })?;
    let policy = tool.policy();
    let arguments = decode_arguments(tool, incoming.arguments).map_err(|message| {
        error_result(
            incoming.request_id.clone(),
            Some(tool),
            Some(policy),
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidArguments,
            message,
        )
    })?;

    Ok(ToolRequest {
        request_id: incoming.request_id,
        tool,
        arguments,
    })
}

/// Validates an untrusted model-produced request with the exact same typed
/// parser used by the executable router. The original JSON is returned only
/// after its tool name and per-tool arguments have passed native validation.
pub(crate) fn validate_model_request(value: Value) -> Result<Value, ToolResult> {
    parse_request(value.clone()).map(|_| value)
}

fn effective_permission(policy: ToolPolicy, preferences: &SkillPreferences) -> ToolPermission {
    if let Some(skill) = policy.required_skill {
        if !preferences.enabled(skill) {
            return ToolPermission::Denied;
        }
    }
    policy.permission
}

pub fn route(value: Value, preferences: &SkillPreferences, nova_enabled: bool) -> ToolResult {
    let request = match parse_request(value) {
        Ok(request) => request,
        Err(result) => return result,
    };
    let policy = request.tool.policy();

    if !nova_enabled {
        return error_result(
            request.request_id,
            Some(request.tool),
            Some(ToolPolicy {
                permission: ToolPermission::Denied,
                ..policy
            }),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "NOVA is off. Tool requests are disabled.",
        );
    }

    let permission = effective_permission(policy, preferences);
    let effective_policy = ToolPolicy {
        permission,
        ..policy
    };

    match permission {
        ToolPermission::Denied => error_result(
            request.request_id,
            Some(request.tool),
            Some(effective_policy),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "This tool is denied by NOVA's native permission policy.",
        ),
        ToolPermission::Confirm => error_result(
            request.request_id,
            Some(request.tool),
            Some(effective_policy),
            ToolResultStatus::ConfirmationRequired,
            ToolErrorCode::ConfirmationRequired,
            "Explicit user confirmation is required. No action was executed.",
        ),
        ToolPermission::Allowed => execute_allowed(request, effective_policy),
    }
}

/// Validate and permission-check a request without executing even an allowed
/// read tool. Compound plans use this to avoid partial side effects during
/// planning and before the whole plan has been explicitly confirmed.
pub(crate) fn preflight(
    value: Value,
    preferences: &SkillPreferences,
    nova_enabled: bool,
) -> ToolResult {
    let request = match parse_request(value) {
        Ok(request) => request,
        Err(result) => return result,
    };
    let policy = request.tool.policy();
    if !nova_enabled {
        return error_result(
            request.request_id,
            Some(request.tool),
            Some(ToolPolicy {
                permission: ToolPermission::Denied,
                ..policy
            }),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "NOVA is off. Tool requests are disabled.",
        );
    }
    let permission = effective_permission(policy, preferences);
    let checked = ToolPolicy {
        permission,
        ..policy
    };
    if permission == ToolPermission::Denied {
        return error_result(
            request.request_id,
            Some(request.tool),
            Some(checked),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "This sequence step is denied by NOVA's native permission policy.",
        );
    }
    ToolResult {
        request_id: request.request_id,
        tool: Some(request.tool),
        category: Some(checked.category),
        risk: Some(checked.risk),
        permission: Some(checked.permission),
        status: ToolResultStatus::ConfirmationRequired,
        data: Some(json!({ "message": "Validated; no action was executed." })),
        error: None,
    }
}

impl ToolName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeveloperOpenProject => "developer.open_project",
            Self::DeveloperStartProject => "developer.start_project",
            Self::DeveloperStopProject => "developer.stop_project",
            Self::DeveloperOpenEditor => "developer.open_editor",
            Self::DeveloperOpenDevUrl => "developer.open_dev_url",
            Self::WorkflowRun => "workflow.run",
            Self::BrowserOpenUrl => "browser.open_url",
            Self::ApplicationOpen => "application.open",
            Self::ApplicationClose => "application.close",
            Self::ApplicationFocus => "application.focus",
            Self::ApplicationIsRunning => "application.is_running",
            Self::ApplicationListRunning => "application.list_running",
            Self::FolderOpen => "folder.open",
            Self::FolderFind => "folder.find",
            Self::FileOpen => "file.open",
            Self::FileFindByName => "file.find_by_name",
            Self::FileRevealInExplorer => "file.reveal_in_explorer",
            Self::FileDelete => "file.delete",
            Self::FolderList => "folder.list",
            Self::SystemGetVolume => "system.get_volume",
            Self::SystemSetVolume => "system.set_volume",
            Self::SystemAdjustVolume => "system.adjust_volume",
            Self::SystemMute => "system.mute",
            Self::SystemUnmute => "system.unmute",
            Self::SystemGetBattery => "system.get_battery",
            Self::SystemGetCpuUsage => "system.get_cpu_usage",
            Self::SystemGetMemoryUsage => "system.get_memory_usage",
            Self::SystemTakeScreenshot => "system.take_screenshot",
            Self::SystemShutdown => "system.shutdown",
            Self::BrowserSearch => "browser.search",
            Self::WindowList => "window.list",
            Self::WindowFocus => "window.focus",
            Self::DeveloperRunScript => "developer.runScript",
            Self::WorkflowPreview => "workflow.preview",
        }
    }
}

fn application_error_code(code: application_control::ApplicationErrorCode) -> ToolErrorCode {
    use application_control::ApplicationErrorCode;
    match code {
        ApplicationErrorCode::UnsupportedPlatform => ToolErrorCode::UnsupportedPlatform,
        ApplicationErrorCode::UnknownApplication => ToolErrorCode::UnknownApplication,
        ApplicationErrorCode::AmbiguousApplication => ToolErrorCode::AmbiguousApplication,
        ApplicationErrorCode::NotInstalled => ToolErrorCode::ApplicationNotInstalled,
        ApplicationErrorCode::NotRunning => ToolErrorCode::ApplicationNotRunning,
        ApplicationErrorCode::LaunchFailed => ToolErrorCode::ApplicationLaunchFailed,
        ApplicationErrorCode::FocusFailed => ToolErrorCode::ApplicationFocusFailed,
        ApplicationErrorCode::CloseFailed => ToolErrorCode::ApplicationCloseFailed,
    }
}

fn display_command(request: &ToolRequest) -> String {
    match &request.arguments {
        ToolArguments::ApplicationOpen(arguments) => {
            format!("Open {}", arguments.application)
        }
        ToolArguments::ApplicationClose(arguments) => {
            format!("Close {}", arguments.application)
        }
        ToolArguments::ApplicationFocus(arguments) => {
            format!("Focus {}", arguments.application)
        }
        ToolArguments::ApplicationIsRunning(arguments) => {
            format!("Check whether {} is running", arguments.application)
        }
        ToolArguments::ApplicationListRunning(_) => "List known running applications".into(),
        ToolArguments::FolderOpen(arguments) => format!("Open folder {}", arguments.folder),
        ToolArguments::FolderFind(arguments) => format!("Find folder {}", arguments.query),
        ToolArguments::FileOpen(arguments) => format!(
            "Open file {}",
            arguments
                .path
                .as_deref()
                .or(arguments.result_id.as_deref())
                .unwrap_or("unknown")
        ),
        ToolArguments::FileFindByName(arguments) => format!("Find file {}", arguments.query),
        ToolArguments::FileRevealInExplorer(arguments) => format!(
            "Reveal file {}",
            arguments
                .path
                .as_deref()
                .or(arguments.result_id.as_deref())
                .unwrap_or("unknown")
        ),
        ToolArguments::SystemGetVolume(_) => "Read system volume".into(),
        ToolArguments::SystemSetVolume(arguments) => {
            format!("Set system volume to {}%", arguments.volume)
        }
        ToolArguments::SystemAdjustVolume(arguments) => {
            format!("Adjust system volume by {:+}%", arguments.delta)
        }
        ToolArguments::SystemMute(_) => "Mute system volume".into(),
        ToolArguments::SystemUnmute(_) => "Unmute system volume".into(),
        ToolArguments::SystemGetBattery(_) => "Read battery status".into(),
        ToolArguments::SystemGetCpuUsage(_) => "Read CPU usage".into(),
        ToolArguments::SystemGetMemoryUsage(_) => "Read memory usage".into(),
        ToolArguments::SystemTakeScreenshot(_) => "Take screenshot".into(),
        ToolArguments::WindowList(_) => "List visible windows".into(),
        ToolArguments::WindowFocus(arguments) => format!(
            "Focus window {}",
            arguments
                .window_id
                .as_deref()
                .or(arguments.title.as_deref())
                .unwrap_or("unknown")
        ),
        _ => request.tool.as_str().to_string(),
    }
}

fn execute_application(request: ToolRequest, policy: ToolPolicy) -> ToolResult {
    let request_id = request.request_id;
    let tool = request.tool;
    let execution = match request.arguments {
        ToolArguments::ApplicationOpen(arguments) => {
            application_control::open(&arguments.application)
        }
        ToolArguments::ApplicationClose(arguments) => {
            application_control::close(&arguments.application)
        }
        ToolArguments::ApplicationFocus(arguments) => {
            application_control::focus(&arguments.application)
        }
        ToolArguments::ApplicationIsRunning(arguments) => {
            application_control::is_running(&arguments.application)
        }
        ToolArguments::ApplicationListRunning(_) => application_control::list_running(),
        _ => {
            return error_result(
                request_id,
                Some(tool),
                Some(policy),
                ToolResultStatus::Unavailable,
                ToolErrorCode::NotImplemented,
                "This is not an application control request.",
            )
        }
    };

    match execution {
        Ok(data) => ToolResult {
            request_id,
            tool: Some(tool),
            category: Some(policy.category),
            risk: Some(policy.risk),
            permission: Some(policy.permission),
            status: ToolResultStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => error_result(
            request_id,
            Some(tool),
            Some(policy),
            ToolResultStatus::Rejected,
            application_error_code(error.code),
            error.message,
        ),
    }
}

fn file_error_code(code: file_control::FileToolErrorCode) -> ToolErrorCode {
    use file_control::FileToolErrorCode;
    match code {
        FileToolErrorCode::UnsupportedPlatform => ToolErrorCode::UnsupportedPlatform,
        FileToolErrorCode::UnknownRoot => ToolErrorCode::UnknownRoot,
        FileToolErrorCode::OutsideApprovedRoots => ToolErrorCode::PathOutsideApprovedRoots,
        FileToolErrorCode::FileNotFound => ToolErrorCode::FileNotFound,
        FileToolErrorCode::FolderNotFound => ToolErrorCode::FolderNotFound,
        FileToolErrorCode::InvalidResultId => ToolErrorCode::InvalidResultId,
        FileToolErrorCode::InvalidProject => ToolErrorCode::InvalidProject,
        FileToolErrorCode::SearchFailed => ToolErrorCode::FileSearchFailed,
        FileToolErrorCode::OpenFailed => ToolErrorCode::FileOpenFailed,
        FileToolErrorCode::RevealFailed => ToolErrorCode::FileRevealFailed,
    }
}

fn execute_file(app: &AppHandle, request: ToolRequest, policy: ToolPolicy) -> ToolResult {
    let request_id = request.request_id;
    let tool = request.tool;
    let execution = match request.arguments {
        ToolArguments::FolderOpen(arguments) => file_control::open_folder(app, &arguments.folder),
        ToolArguments::FolderFind(arguments) => file_control::find_folders(app, &arguments),
        ToolArguments::FileOpen(arguments) => file_control::open_file(app, &arguments),
        ToolArguments::FileFindByName(arguments) => file_control::find_files(app, &arguments),
        ToolArguments::FileRevealInExplorer(arguments) => {
            file_control::reveal_file(app, &arguments)
        }
        _ => {
            return error_result(
                request_id,
                Some(tool),
                Some(policy),
                ToolResultStatus::Unavailable,
                ToolErrorCode::NotImplemented,
                "This is not a supported file or folder request.",
            )
        }
    };

    match execution {
        Ok(data) => ToolResult {
            request_id,
            tool: Some(tool),
            category: Some(policy.category),
            risk: Some(policy.risk),
            permission: Some(policy.permission),
            status: ToolResultStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => error_result(
            request_id,
            Some(tool),
            Some(policy),
            ToolResultStatus::Rejected,
            file_error_code(error.code),
            error.message,
        ),
    }
}
fn system_error_code(code: system_control::SystemControlErrorCode) -> ToolErrorCode {
    use system_control::SystemControlErrorCode;
    match code {
        SystemControlErrorCode::UnsupportedPlatform => ToolErrorCode::UnsupportedPlatform,
        SystemControlErrorCode::InvalidArguments => ToolErrorCode::InvalidArguments,
        SystemControlErrorCode::VolumeUnavailable => ToolErrorCode::VolumeUnavailable,
        SystemControlErrorCode::SystemInfoUnavailable => ToolErrorCode::SystemInfoUnavailable,
        SystemControlErrorCode::ScreenshotFailed => ToolErrorCode::ScreenshotFailed,
    }
}

fn window_error_code(code: window_control::WindowControlErrorCode) -> ToolErrorCode {
    use window_control::WindowControlErrorCode;
    match code {
        WindowControlErrorCode::UnsupportedPlatform => ToolErrorCode::UnsupportedPlatform,
        WindowControlErrorCode::InvalidArguments => ToolErrorCode::InvalidWindow,
        WindowControlErrorCode::WindowNotFound => ToolErrorCode::WindowNotFound,
        WindowControlErrorCode::AmbiguousWindow => ToolErrorCode::AmbiguousWindow,
        WindowControlErrorCode::FocusFailed => ToolErrorCode::WindowFocusFailed,
    }
}

fn execute_system(app: Option<&AppHandle>, request: ToolRequest, policy: ToolPolicy) -> ToolResult {
    let request_id = request.request_id;
    let tool = request.tool;
    let execution = match request.arguments {
        ToolArguments::SystemGetVolume(_) => system_control::get_volume(),
        ToolArguments::SystemSetVolume(arguments) => system_control::set_volume(arguments.volume),
        ToolArguments::SystemAdjustVolume(arguments) => {
            system_control::adjust_volume(arguments.delta)
        }
        ToolArguments::SystemMute(_) => system_control::mute(),
        ToolArguments::SystemUnmute(_) => system_control::unmute(),
        ToolArguments::SystemGetBattery(_) => system_control::get_battery(),
        ToolArguments::SystemGetCpuUsage(_) => system_control::get_cpu_usage(),
        ToolArguments::SystemGetMemoryUsage(_) => system_control::get_memory_usage(),
        ToolArguments::SystemTakeScreenshot(_) => match app {
            Some(app) => system_control::take_screenshot(app),
            None => Err(system_control::SystemControlError {
                code: system_control::SystemControlErrorCode::ScreenshotFailed,
                message: "Screenshots require a confirmed desktop command context.".into(),
            }),
        },
        _ => {
            return error_result(
                request_id,
                Some(tool),
                Some(policy),
                ToolResultStatus::Unavailable,
                ToolErrorCode::NotImplemented,
                "This is not a supported system request.",
            )
        }
    };

    match execution {
        Ok(data) => ToolResult {
            request_id,
            tool: Some(tool),
            category: Some(policy.category),
            risk: Some(policy.risk),
            permission: Some(policy.permission),
            status: ToolResultStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => error_result(
            request_id,
            Some(tool),
            Some(policy),
            ToolResultStatus::Rejected,
            system_error_code(error.code),
            error.message,
        ),
    }
}

fn execute_window(request: ToolRequest, policy: ToolPolicy) -> ToolResult {
    let request_id = request.request_id;
    let tool = request.tool;
    let execution = match request.arguments {
        ToolArguments::WindowList(_) => window_control::list_windows(),
        ToolArguments::WindowFocus(arguments) => window_control::focus_window(&arguments),
        _ => {
            return error_result(
                request_id,
                Some(tool),
                Some(policy),
                ToolResultStatus::Unavailable,
                ToolErrorCode::NotImplemented,
                "This is not a supported window request.",
            )
        }
    };

    match execution {
        Ok(data) => ToolResult {
            request_id,
            tool: Some(tool),
            category: Some(policy.category),
            risk: Some(policy.risk),
            permission: Some(policy.permission),
            status: ToolResultStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => error_result(
            request_id,
            Some(tool),
            Some(policy),
            ToolResultStatus::Rejected,
            window_error_code(error.code),
            error.message,
        ),
    }
}

fn execute_allowed(request: ToolRequest, policy: ToolPolicy) -> ToolResult {
    match request.arguments {
        ToolArguments::ApplicationIsRunning(_) | ToolArguments::ApplicationListRunning(_) => {
            execute_application(request, policy)
        }
        ToolArguments::SystemGetVolume(_)
        | ToolArguments::SystemGetBattery(_)
        | ToolArguments::SystemGetCpuUsage(_)
        | ToolArguments::SystemGetMemoryUsage(_) => execute_system(None, request, policy),
        ToolArguments::WindowList(_) => execute_window(request, policy),
        ToolArguments::WorkflowPreview(arguments) => ToolResult {
            request_id: request.request_id,
            tool: Some(request.tool),
            category: Some(policy.category),
            risk: Some(policy.risk),
            permission: Some(policy.permission),
            status: ToolResultStatus::Completed,
            data: Some(json!({ "stepCount": arguments.steps.len() })),
            error: None,
        },
        _ => error_result(
            request.request_id,
            Some(request.tool),
            Some(policy),
            ToolResultStatus::Unavailable,
            ToolErrorCode::NotImplemented,
            "This typed tool has no native implementation. No action was executed.",
        ),
    }
}

fn record_application_result(app: &AppHandle, request: &ToolRequest, result: &ToolResult, duration: u64) {
    if request.tool.policy().category != ToolCategory::Application {
        return;
    }
    if let Err(error) =
        command_history::record_duration(app, request.tool.as_str(), display_command(request), result, duration)
    {
        eprintln!("Could not record NOVA command history: {error}");
    }
}

#[tauri::command]
pub fn execute_confirmed_application_tool(
    app: AppHandle,
    request: Value,
    confirmed: bool,
) -> ToolResult {
    let started = std::time::Instant::now();
    let parsed = match parse_request(request) {
        Ok(request) => request,
        Err(result) => return result,
    };
    let policy = parsed.tool.policy();
    if policy.category != ToolCategory::Application {
        return error_result(
            parsed.request_id,
            Some(parsed.tool),
            Some(policy),
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidTool,
            "Only application tools are accepted by this command.",
        );
    }

    let nova_enabled = match settings::nova_enabled(&app) {
        Ok(enabled) => enabled,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let preferences = match settings::skill_preferences(&app) {
        Ok(preferences) => preferences,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };

    let permission = if nova_enabled {
        effective_permission(policy, &preferences)
    } else {
        ToolPermission::Denied
    };
    let effective_policy = ToolPolicy {
        permission,
        ..policy
    };
    let result = match permission {
        ToolPermission::Denied => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "Application tools are disabled by NOVA's native settings.",
        ),
        ToolPermission::Confirm if !confirmed => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::ConfirmationRequired,
            ToolErrorCode::ConfirmationRequired,
            "Explicit user confirmation is required. No action was executed.",
        ),
        ToolPermission::Confirm | ToolPermission::Allowed => {
            execute_application(parsed.clone(), effective_policy)
        }
    };
    record_application_result(&app, &parsed, &result, started.elapsed().as_millis() as u64);
    result
}

fn is_supported_file_tool(tool: ToolName) -> bool {
    matches!(
        tool,
        ToolName::FolderOpen
            | ToolName::FolderFind
            | ToolName::FileOpen
            | ToolName::FileFindByName
            | ToolName::FileRevealInExplorer
    )
}

#[tauri::command]
pub fn execute_confirmed_file_tool(app: AppHandle, request: Value, confirmed: bool) -> ToolResult {
    let started = std::time::Instant::now();
    let parsed = match parse_request(request) {
        Ok(request) => request,
        Err(result) => return result,
    };
    let policy = parsed.tool.policy();
    if !is_supported_file_tool(parsed.tool) {
        return error_result(
            parsed.request_id,
            Some(parsed.tool),
            Some(policy),
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidTool,
            "Only implemented file and folder tools are accepted by this command.",
        );
    }

    let nova_enabled = match settings::nova_enabled(&app) {
        Ok(enabled) => enabled,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let preferences = match settings::skill_preferences(&app) {
        Ok(preferences) => preferences,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let permission = if nova_enabled {
        effective_permission(policy, &preferences)
    } else {
        ToolPermission::Denied
    };
    let effective_policy = ToolPolicy {
        permission,
        ..policy
    };
    let result = match permission {
        ToolPermission::Denied => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "File and folder tools are disabled by NOVA's native settings.",
        ),
        ToolPermission::Confirm if !confirmed => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::ConfirmationRequired,
            ToolErrorCode::ConfirmationRequired,
            "Explicit user confirmation is required. No action was executed.",
        ),
        ToolPermission::Confirm | ToolPermission::Allowed => {
            execute_file(&app, parsed.clone(), effective_policy)
        }
    };

    if let Err(error) = command_history::record_duration(
        &app,
        parsed.tool.as_str(),
        display_command(&parsed),
        &result,
        started.elapsed().as_millis() as u64,
    ) {
        eprintln!("Could not record NOVA command history: {error}");
    }
    result
}

fn is_supported_system_or_window_tool(tool: ToolName) -> bool {
    matches!(
        tool,
        ToolName::SystemGetVolume
            | ToolName::SystemSetVolume
            | ToolName::SystemAdjustVolume
            | ToolName::SystemMute
            | ToolName::SystemUnmute
            | ToolName::SystemGetBattery
            | ToolName::SystemGetCpuUsage
            | ToolName::SystemGetMemoryUsage
            | ToolName::SystemTakeScreenshot
            | ToolName::WindowList
            | ToolName::WindowFocus
    )
}

#[tauri::command]
pub fn execute_confirmed_system_tool(
    app: AppHandle,
    request: Value,
    confirmed: bool,
) -> ToolResult {
    let started = std::time::Instant::now();
    let parsed = match parse_request(request) {
        Ok(request) => request,
        Err(result) => return result,
    };
    let policy = parsed.tool.policy();
    if !is_supported_system_or_window_tool(parsed.tool) {
        return error_result(
            parsed.request_id,
            Some(parsed.tool),
            Some(policy),
            ToolResultStatus::Rejected,
            ToolErrorCode::InvalidTool,
            "Only implemented system and window tools are accepted by this command.",
        );
    }

    let nova_enabled = match settings::nova_enabled(&app) {
        Ok(enabled) => enabled,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let preferences = match settings::skill_preferences(&app) {
        Ok(preferences) => preferences,
        Err(error) => {
            return error_result(
                parsed.request_id,
                Some(parsed.tool),
                Some(policy),
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let permission = if nova_enabled {
        effective_permission(policy, &preferences)
    } else {
        ToolPermission::Denied
    };
    let effective_policy = ToolPolicy {
        permission,
        ..policy
    };
    let result = match permission {
        ToolPermission::Denied => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::Denied,
            ToolErrorCode::PermissionDenied,
            "System and window tools are disabled by NOVA's native settings.",
        ),
        ToolPermission::Confirm if !confirmed => error_result(
            parsed.request_id.clone(),
            Some(parsed.tool),
            Some(effective_policy),
            ToolResultStatus::ConfirmationRequired,
            ToolErrorCode::ConfirmationRequired,
            "Explicit user confirmation is required. No action was executed.",
        ),
        ToolPermission::Confirm | ToolPermission::Allowed => match effective_policy.category {
            ToolCategory::System => execute_system(Some(&app), parsed.clone(), effective_policy),
            ToolCategory::Window => execute_window(parsed.clone(), effective_policy),
            _ => error_result(
                parsed.request_id.clone(),
                Some(parsed.tool),
                Some(effective_policy),
                ToolResultStatus::Unavailable,
                ToolErrorCode::NotImplemented,
                "This is not a supported system or window request.",
            ),
        },
    };

    if let Err(error) = command_history::record_duration(
        &app,
        parsed.tool.as_str(),
        display_command(&parsed),
        &result,
        started.elapsed().as_millis() as u64,
    ) {
        eprintln!("Could not record NOVA command history: {error}");
    }
    result
}
#[tauri::command]
pub fn route_tool_request(app: AppHandle, request: Value) -> ToolResult {
    let preferences = match settings::skill_preferences(&app) {
        Ok(preferences) => preferences,
        Err(error) => {
            return error_result(
                None,
                None,
                None,
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    let nova_enabled = match settings::nova_enabled(&app) {
        Ok(enabled) => enabled,
        Err(error) => {
            return error_result(
                None,
                None,
                None,
                ToolResultStatus::Rejected,
                ToolErrorCode::SettingsUnavailable,
                error,
            )
        }
    };
    route(request, &preferences, nova_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, arguments: Value) -> Value {
        json!({
            "requestId": "test-request",
            "tool": tool,
            "arguments": arguments
        })
    }

    #[test]
    fn valid_request_reaches_confirmation_boundary() {
        let result = route(
            request(
                "application.open",
                json!({ "application": "Visual Studio Code" }),
            ),
            &SkillPreferences::default(),
            true,
        );

        assert_eq!(result.status, ToolResultStatus::ConfirmationRequired);
        assert_eq!(result.permission, Some(ToolPermission::Confirm));
        assert_eq!(result.risk, Some(ToolRiskLevel::Safe));
    }

    #[test]
    fn sequence_preflight_never_executes_allowed_read_tools() {
        let mut preferences = SkillPreferences::default();
        preferences.system = true;
        let result = preflight(
            json!({
                "requestId": "preflight-read",
                "tool": "system.get_battery",
                "arguments": {}
            }),
            &preferences,
            true,
        );
        assert_eq!(result.status, ToolResultStatus::ConfirmationRequired);
        assert_eq!(
            result
                .data
                .as_ref()
                .and_then(|data| data.get("message"))
                .and_then(Value::as_str),
            Some("Validated; no action was executed.")
        );
    }

    #[test]
    fn unknown_tool_is_rejected() {
        let result = route(
            request("shell.execute", json!({ "command": "whoami" })),
            &SkillPreferences::default(),
            true,
        );

        assert_eq!(result.status, ToolResultStatus::Rejected);
        assert_eq!(
            result.error.map(|error| error.code),
            Some(ToolErrorCode::InvalidTool)
        );
    }

    #[test]
    fn invalid_and_extra_arguments_are_rejected() {
        let empty = route(
            request("application.open", json!({ "application": " " })),
            &SkillPreferences::default(),
            true,
        );
        let extra = route(
            request(
                "application.open",
                json!({ "application": "Code", "command": "unexpected" }),
            ),
            &SkillPreferences::default(),
            true,
        );

        assert_eq!(empty.status, ToolResultStatus::Rejected);
        assert_eq!(extra.status, ToolResultStatus::Rejected);
    }

    #[test]
    fn relative_volume_requires_a_nonzero_bounded_delta() {
        let mut preferences = SkillPreferences::default();
        preferences.system = true;

        for delta in [0, -101, 101] {
            let result = route(
                request("system.adjust_volume", json!({ "delta": delta })),
                &preferences,
                true,
            );
            assert_eq!(result.status, ToolResultStatus::Rejected);
        }
        let valid = route(
            request("system.adjust_volume", json!({ "delta": -15 })),
            &preferences,
            true,
        );
        assert_eq!(valid.status, ToolResultStatus::ConfirmationRequired);
    }

    #[test]
    fn disabled_skill_denies_tool() {
        let mut preferences = SkillPreferences::default();
        preferences.apps = false;
        let result = route(
            request("application.open", json!({ "application": "Code" })),
            &preferences,
            true,
        );

        assert_eq!(result.status, ToolResultStatus::Denied);
        assert_eq!(result.permission, Some(ToolPermission::Denied));
    }

    #[test]
    fn destructive_tool_stays_denied_when_preference_is_enabled() {
        let mut preferences = SkillPreferences::default();
        preferences.files = true;
        let result = route(
            request("file.delete", json!({ "path": "C:\\temp\\example.txt" })),
            &preferences,
            true,
        );

        assert_eq!(result.status, ToolResultStatus::Denied);
        assert_eq!(result.risk, Some(ToolRiskLevel::Destructive));
    }

    #[test]
    fn nova_off_denies_all_tools() {
        let result = route(
            request("workflow.preview", json!({ "steps": ["one"] })),
            &SkillPreferences::default(),
            false,
        );

        assert_eq!(result.status, ToolResultStatus::Denied);
    }

    #[test]
    fn harmless_workflow_preview_proves_allowed_routing() {
        let result = route(
            request("workflow.preview", json!({ "steps": ["one", "two"] })),
            &SkillPreferences::default(),
            true,
        );

        assert_eq!(result.status, ToolResultStatus::Completed);
        assert_eq!(result.permission, Some(ToolPermission::Allowed));
        assert_eq!(result.data, Some(json!({ "stepCount": 2 })));
    }
}


pub(crate) enum RegisteredOrigin { DashboardConfirmed, Voice, ApprovedWorkflow }
// Only the saved-workflow runner may call this; each step has been saved by the user.
pub fn execute_workflow_step(app: AppHandle, request: Value) -> ToolResult {
    match request.get("tool").and_then(Value::as_str).unwrap_or("") {
        tool if tool.starts_with("application.") => execute_confirmed_application_tool(app,request,true),
        tool if tool.starts_with("file.") || tool.starts_with("folder.") => execute_confirmed_file_tool(app,request,true),
        tool if tool.starts_with("system.") || tool.starts_with("window.") => execute_confirmed_system_tool(app,request,true),
        _ => execute_registered(&app,request,RegisteredOrigin::ApprovedWorkflow),
    }
}
pub(crate) fn execute_registered(app: &AppHandle, request: Value, origin: RegisteredOrigin) -> ToolResult {
    let parsed = match parse_request(request.clone()) { Ok(p) => p, Err(r) => return r };
    let preferences = match settings::skill_preferences(app) { Ok(p) => p, Err(e) => return error_result(parsed.request_id,Some(parsed.tool),Some(parsed.tool.policy()),ToolResultStatus::Rejected,ToolErrorCode::SettingsUnavailable,e) };
    let checked = preflight(request.clone(), &preferences, settings::nova_enabled(app).unwrap_or(false));
    if checked.status != ToolResultStatus::ConfirmationRequired { return checked; }
    // This check is in the execution boundary, not merely in a caller or the UI.
    if matches!(origin, RegisteredOrigin::Voice) && !crate::developer::voice_execution_approved(app, &request) {
        return error_result(parsed.request_id,Some(parsed.tool),Some(parsed.tool.policy()),ToolResultStatus::ConfirmationRequired,ToolErrorCode::ConfirmationRequired,"Approve this saved profile or workflow for voice execution in the dashboard first.");
    }
    let started = std::time::Instant::now();
    let execution = match &parsed.arguments {
        ToolArguments::RegisteredTarget(args) if parsed.tool == ToolName::WorkflowRun => crate::developer::run_workflow(app,&args.name),
        ToolArguments::RegisteredTarget(args) => crate::developer::project_action(app,parsed.tool.as_str(),&args.name),
        ToolArguments::BrowserOpenUrl(args) => crate::developer::open_url(app,&args.url),
        _ => Err("Only registered project, workflow and URL actions are accepted.".into()),
    };
    let result = match execution {
        Ok(data) => ToolResult { data:Some(data), error:None, status:ToolResultStatus::Completed, ..checked },
        Err(message) => error_result(parsed.request_id.clone(),Some(parsed.tool),Some(parsed.tool.policy()),ToolResultStatus::Rejected,ToolErrorCode::InvalidArguments,message),
    };
    let _ = command_history::record_duration(app,parsed.tool.as_str(),format!("{} {}",parsed.tool.as_str(),match &parsed.arguments { ToolArguments::RegisteredTarget(a)=>a.name.as_str(), ToolArguments::BrowserOpenUrl(a)=>a.url.as_str(), _=>"" }),&result,started.elapsed().as_millis() as u64);
    result
}
#[tauri::command]
pub async fn execute_registered_tool(window: tauri::WebviewWindow, request: Value, confirmed: bool) -> ToolResult {
    if window.label() != "main" || !confirmed { return error_result(None,None,None,ToolResultStatus::ConfirmationRequired,ToolErrorCode::ConfirmationRequired,"Confirm registered actions in the dashboard."); }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || execute_registered(&app,request,RegisteredOrigin::DashboardConfirmed)).await.unwrap_or_else(|e| {
        error_result(None,None,None,ToolResultStatus::Rejected,ToolErrorCode::InvalidArguments,format!("Registered action failed: {e}"))
    })
}


#[cfg(test)] mod project_security_tests {
    use super::*;
    #[test] fn registered_actions_require_skill_and_confirmation() {
        let request=json!({"tool":"developer.start_project","arguments":{"name":"ProctorX"}});
        assert_eq!(route(request.clone(),&SkillPreferences::default(),true).status,ToolResultStatus::Denied);
        let mut preferences=SkillPreferences::default();preferences.scripts=true;
        assert_eq!(route(request.clone(),&preferences,true).status,ToolResultStatus::ConfirmationRequired);
        assert_eq!(route(request,&preferences,false).status,ToolResultStatus::Denied);
    }
    #[test] fn model_cannot_supply_commands_or_voice_approval() {
        for arguments in [json!({"name":"ProctorX","command":"npm run dev"}),json!({"name":"ProctorX","voiceEnabled":true}),json!({"name":"ProctorX","path":"C:/evil"})] {
            assert!(validate_model_request(json!({"tool":"developer.start_project","arguments":arguments})).is_err());
        }
    }
}
