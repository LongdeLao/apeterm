use std::{
    env, fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};

use crate::features::portfolio::PortfolioSnapshot;

const LOCAL_PYTHON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/bin/python");
const LOCAL_BRIDGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/broker/trade_republic.py");

pub fn available() -> bool {
    let mut command = Command::new(python_command());
    configure_python_environment(&mut command);
    command
        .args(["-c", "import pytr"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn connect() -> io::Result<()> {
    ensure_available()?;
    let mut command = Command::new(python_command());
    configure_python_environment(&mut command);
    let status = command.arg(bridge_path()).arg("connect").status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("pytr login failed"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginStartResult {
    Connected,
    CodeRequired {
        process_id: String,
        countdown: Option<u64>,
    },
}

#[derive(Serialize)]
struct LoginStartRequest<'a> {
    phone: &'a str,
    pin: &'a str,
}

#[derive(Serialize)]
struct LoginCompleteRequest<'a> {
    phone: &'a str,
    pin: &'a str,
    process_id: &'a str,
    code: &'a str,
}

pub fn login_start(phone: &str, pin: &str) -> io::Result<LoginStartResult> {
    let bytes = run_json_bridge(
        "login-start",
        &LoginStartRequest {
            phone: phone.trim(),
            pin: pin.trim(),
        },
    )?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub fn login_complete(phone: &str, pin: &str, process_id: &str, code: &str) -> io::Result<()> {
    let bytes = run_json_bridge(
        "login-complete",
        &LoginCompleteRequest {
            phone: phone.trim(),
            pin: pin.trim(),
            process_id: process_id.trim(),
            code: code.trim(),
        },
    )?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if value.get("status").and_then(serde_json::Value::as_str) == Some("connected") {
        Ok(())
    } else {
        Err(io::Error::other("Trade Republic login did not complete"))
    }
}

fn run_json_bridge(command_name: &str, request: &impl Serialize) -> io::Result<Vec<u8>> {
    ensure_available()?;
    let mut command = Command::new(python_command());
    configure_python_environment(&mut command);
    let mut child = command
        .arg(bridge_path())
        .arg(command_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err(io::Error::other("failed to open broker bridge stdin"));
        };
        serde_json::to_writer(&mut *stdin, request).map_err(io::Error::other)?;
        stdin.write_all(b"\n")?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(io::Error::other(if stderr.is_empty() {
            "Trade Republic login failed".to_string()
        } else {
            stderr
        }))
    }
}

pub fn sync(output: &Path) -> io::Result<PortfolioSnapshot> {
    ensure_available()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut command = Command::new(python_command());
    configure_python_environment(&mut command);
    let result = command
        .arg(bridge_path())
        .arg("sync")
        .arg("--output")
        .arg(output)
        .output()?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            "Trade Republic sync failed; open Portfolio and press c to connect".to_string()
        } else {
            stderr
        }));
    }
    let bytes = fs::read(output)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

fn configure_python_environment(command: &mut Command) {
    if let Some(parent) = Path::new(&python_command()).parent()
        && !parent.as_os_str().is_empty()
    {
        let mut paths = vec![parent.to_path_buf()];
        if let Some(path) = env::var_os("PATH") {
            paths.extend(env::split_paths(&path));
        }
        if let Ok(path) = env::join_paths(paths) {
            command.env("PATH", path);
        }
    }
}

fn ensure_available() -> io::Result<()> {
    if available() {
        Ok(())
    } else {
        Err(io::Error::other(
            "optional pytr dependency is missing; install `broker/requirements.txt` or reinstall with INSTALL_BROKER_DEPS=1",
        ))
    }
}

fn python_command() -> String {
    if let Ok(value) = env::var("APETERM_PYTHON")
        && !value.trim().is_empty()
    {
        value
    } else if Path::new(LOCAL_PYTHON).exists() {
        LOCAL_PYTHON.to_string()
    } else {
        "python3".to_string()
    }
}

fn bridge_path() -> PathBuf {
    if let Ok(dir) = env::var("APETERM_BROKER_DIR") {
        let path = Path::new(dir.trim()).join("trade_republic.py");
        if path.exists() {
            return path;
        }
    }
    PathBuf::from(LOCAL_BRIDGE)
}
