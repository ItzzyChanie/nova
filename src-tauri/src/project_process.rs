//! Owned process trees. Start suspended, assign to a kill-on-close job, then resume.
use std::path::{Path, PathBuf};

pub fn executable(name: &str) -> Result<PathBuf, String> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|p| p.is_absolute())
        .map(|p| p.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| format!("{name} is unavailable. Install it and restart NOVA."))
}

pub fn command_spec(command: &str) -> Result<Option<(String, Vec<String>)>, String> {
    if command.len() > 256 || command.chars().any(char::is_control) { return Err("Command must be one line of at most 256 characters.".into()); }
    let words: Vec<_> = command.split_whitespace().collect();
    match words.as_slice() {
        [] => Ok(None),
        ["npm", "run", script] if script.len() <= 80 && !script.starts_with('-') && script.chars().all(|c| c.is_ascii_alphanumeric() || "_:-.".contains(c)) =>
            Ok(Some(("npm".into(), vec![script.to_string()]))),
        ["dotnet", "run"] => Ok(Some(("dotnet".into(), vec!["run".into()]))),
        _ => Err("Use npm run <script> or dotnet run. Shell operators, inline scripts and extra arguments are not allowed.".into()),
    }
}

pub fn resolve(command: &str, cwd: &Path) -> Result<Option<(PathBuf, Vec<String>)>, String> {
    let Some((runner, args)) = command_spec(command)? else {
        return Ok(None);
    };
    if runner == "npm" {
        let package: serde_json::Value = serde_json::from_slice(
            &std::fs::read(cwd.join("package.json"))
                .map_err(|e| format!("Cannot read package.json: {e}"))?,
        )
        .map_err(|e| e.to_string())?;
        if package
            .get("scripts")
            .and_then(|v| v.get(&args[0]))
            .and_then(|v| v.as_str())
            .is_none()
        {
            return Err(format!(
                "npm script '{}' is missing from package.json.",
                args[0]
            ));
        }
        let node = executable("node.exe")?;
        let cli = node
            .parent()
            .ok_or("Invalid Node installation.")?
            .join("node_modules/npm/bin/npm-cli.js");
        if !cli.is_file() {
            return Err(
                "npm-cli.js was not found beside node.exe. Install Node.js with npm.".into(),
            );
        }
        Ok(Some((
            node,
            vec![cli.to_string_lossy().into(), "run".into(), args[0].clone()],
        )))
    } else {
        Ok(Some((executable("dotnet.exe")?, args)))
    }
}

// Windows CRT argument quoting: each argument remains a single argument, including trailing slashes.
fn quote(value: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for c in value.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        result.extend(std::iter::repeat_n(
            '\\',
            if c == '"' { slashes * 2 + 1 } else { slashes },
        ));
        slashes = 0;
        result.push(c);
    }
    result.extend(std::iter::repeat_n('\\', slashes * 2));
    result.push('"');
    result
}

#[cfg(windows)]
pub struct ProjectProcess {
    job: windows_sys::Win32::Foundation::HANDLE,
    process: windows_sys::Win32::Foundation::HANDLE,
}
#[cfg(windows)]
unsafe impl Send for ProjectProcess {} // Owned handles; accessed only behind the project mutex.
#[cfg(windows)]
impl ProjectProcess {
    pub fn start(program: &Path, args: &[String], cwd: &Path) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::{JobObjects::*, Threading::*},
        };
        let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<_>>();
        let program_w = wide(program.as_os_str());
        let cwd_w = wide(cwd.as_os_str());
        let text = std::iter::once(program.to_string_lossy().to_string())
            .chain(args.iter().cloned())
            .map(|s| quote(&s))
            .collect::<Vec<_>>()
            .join(" ");
        let mut command = wide(std::ffi::OsStr::new(&text));
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
            {
                CloseHandle(job);
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut startup: STARTUPINFOW = std::mem::zeroed();
            startup.cb = std::mem::size_of_val(&startup) as u32;
            let mut process: PROCESS_INFORMATION = std::mem::zeroed();
            if CreateProcessW(
                program_w.as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                CREATE_SUSPENDED | CREATE_NO_WINDOW,
                std::ptr::null(),
                cwd_w.as_ptr(),
                &startup,
                &mut process,
            ) == 0
            {
                let error = std::io::Error::last_os_error().to_string();
                CloseHandle(job);
                return Err(error);
            }
            if AssignProcessToJobObject(job, process.hProcess) == 0
                || ResumeThread(process.hThread) == u32::MAX
            {
                let error = std::io::Error::last_os_error().to_string();
                TerminateProcess(process.hProcess, 1);
                CloseHandle(process.hThread);
                CloseHandle(process.hProcess);
                CloseHandle(job);
                return Err(error);
            }
            CloseHandle(process.hThread);
            Ok(Self {
                job,
                process: process.hProcess,
            })
        }
    }
    pub fn running(&self) -> bool {
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
            QueryInformationJobObject(
                self.job,
                JobObjectBasicAccountingInformation,
                &mut info as *mut _ as _,
                std::mem::size_of_val(&info) as u32,
                std::ptr::null_mut(),
            ) != 0
                && info.ActiveProcesses > 0
        }
    }
}
#[cfg(windows)]
impl Drop for ProjectProcess {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.job);
            windows_sys::Win32::Foundation::CloseHandle(self.process);
        }
    }
}
#[cfg(not(windows))]
pub struct ProjectProcess;
#[cfg(not(windows))]
impl ProjectProcess {
    pub fn start(_: &Path, _: &[String], _: &Path) -> Result<Self, String> {
        Err("Development processes require Windows.".into())
    }
    pub fn running(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_reject_shell_and_model_payloads() {
        for s in [
            "npm run dev && whoami",
            "powershell -c evil",
            "cmd /c dir",
            "npm exec evil",
            "npm run --help",
            "npm run dev;evil",
            "node -e evil",
        ] {
            assert!(command_spec(s).is_err(), "{s}");
        }
        assert!(command_spec("npm run dev:frontend").unwrap().is_some());
        assert!(command_spec("dotnet run").unwrap().is_some());
        assert!(command_spec("").unwrap().is_none());
    }
    #[test]
    fn quote_keeps_spaces_and_quotes_in_one_argument() {
        assert_eq!(quote("hello world"), "\"hello world\"");
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote("path\\"), "\"path\\\\\"");
    }
    #[test]
    #[cfg(windows)]
    #[ignore = "starts a local Node process and verifies owned process-tree cleanup"]
    fn owned_process_tree_stops() {
        let node = executable("node.exe").unwrap();
        let process = ProjectProcess::start(
            &node,
            &["-e".into(), "setInterval(()=>{},1000)".into()],
            &std::env::temp_dir(),
        )
        .unwrap();
        assert!(process.running());
        unsafe {
            use windows_sys::Win32::{Foundation::{CloseHandle,WAIT_OBJECT_0},System::Threading::{OpenProcess,GetProcessId,WaitForSingleObject,PROCESS_SYNCHRONIZE}};
            let observer=OpenProcess(PROCESS_SYNCHRONIZE,0,GetProcessId(process.process));
            assert!(!observer.is_null());
            drop(process);
            assert_eq!(WaitForSingleObject(observer,5000),WAIT_OBJECT_0);
            CloseHandle(observer);
        }
    }
    #[test]
    #[cfg(windows)]
    #[ignore = "runs an isolated npm fixture on a loopback port and verifies descendant cleanup"]
    fn configured_npm_script_starts_and_stops_server() {
        use windows_sys::Win32::{Foundation::{CloseHandle, WAIT_OBJECT_0}, System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE}};
        let root = std::env::temp_dir().join(format!("nova-npm-smoke-{}",std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"),r#"{"private":true,"scripts":{"dev":"node server.cjs"}}"#).unwrap();
        std::fs::write(root.join("server.cjs"),r#"const fs=require('fs');const server=require('http').createServer((req,res)=>res.end('NOVA test'));server.listen(0,'127.0.0.1',()=>fs.writeFileSync('ready.json',JSON.stringify({pid:process.pid,port:server.address().port})));"#).unwrap();
        let (program,args)=resolve("npm run dev",&root).unwrap().unwrap();
        let process=ProjectProcess::start(&program,&args,&root).unwrap();
        let started=std::time::Instant::now();
        while !root.join("ready.json").exists() && started.elapsed()<std::time::Duration::from_secs(10) { std::thread::sleep(std::time::Duration::from_millis(50)); }
        let ready:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("ready.json")).expect("npm server should publish readiness")).unwrap();
        let address=std::net::SocketAddr::from(([127,0,0,1],ready["port"].as_u64().unwrap() as u16));
        assert!(std::net::TcpStream::connect_timeout(&address,std::time::Duration::from_secs(1)).is_ok());
        unsafe {
            let observer=OpenProcess(PROCESS_SYNCHRONIZE,0,ready["pid"].as_u64().unwrap() as u32);
            assert!(!observer.is_null()); drop(process);
            assert_eq!(WaitForSingleObject(observer,5000),WAIT_OBJECT_0,"NOVA must terminate the npm child server, not only the launcher");
            CloseHandle(observer);
        }
        for name in ["package.json","server.cjs","ready.json"] {std::fs::remove_file(root.join(name)).unwrap();}
        std::fs::remove_dir(root).unwrap();
    }

}
