use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::features::portfolio::PortfolioSnapshot;

const LOCAL_PYTHON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/bin/python");
const LOCAL_BRIDGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/broker/trade_republic.py");

pub fn available() -> bool {
    Command::new(python_command())
        .args(["-c", "import pytr"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn connect() -> io::Result<()> {
    ensure_available()?;
    let status = Command::new(python_command())
        .arg(bridge_path())
        .arg("connect")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("pytr login failed"))
    }
}

pub fn sync(output: &Path) -> io::Result<PortfolioSnapshot> {
    ensure_available()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let result = Command::new(python_command())
        .arg(bridge_path())
        .arg("sync")
        .arg("--output")
        .arg(output)
        .output()?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            "Trade Republic sync failed; run `apeterm broker connect` in a terminal".to_string()
        } else {
            stderr
        }));
    }
    let bytes = fs::read(output)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
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
