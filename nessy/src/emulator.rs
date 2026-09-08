use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum EmulatorBackend {
    Mame,
    Fceux,
}

impl EmulatorBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mame => "mame",
            Self::Fceux => "fceux",
        }
    }
}

impl FromStr for EmulatorBackend {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw {
            "mame" => Ok(Self::Mame),
            "fceux" => Ok(Self::Fceux),
            _ => bail!(
                "unsupported emulator backend '{}', use 'mame' or 'fceux'",
                raw
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeadlessRun {
    pub backend: EmulatorBackend,
    pub rom: PathBuf,
    pub script: Option<PathBuf>,
    pub timeout_seconds: u64,
    pub audio: bool,
    pub cwd: Option<PathBuf>,
    pub extra_args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeadlessOutcome {
    pub backend: String,
    pub command_line: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub log_file: Option<String>,
}

/// Runs one emulator process headlessly with timeout enforcement.
pub fn run_headless(run: &HeadlessRun) -> Result<HeadlessOutcome> {
    if !run.rom.exists() {
        bail!("ROM not found: {}", run.rom.display());
    }

    let mut cmd = match run.backend {
        EmulatorBackend::Mame => {
            let mut command = Command::new("mame");
            command
                .arg("nes")
                .arg("-cart")
                .arg(&run.rom)
                .arg("-video")
                .arg("none");
            if !run.audio {
                command.arg("-sound").arg("none");
            }
            command
                .arg("-nothrottle")
                .arg("-seconds_to_run")
                .arg(run.timeout_seconds.to_string());
            if let Some(script) = &run.script {
                command
                    .arg("-autoboot_delay")
                    .arg("1")
                    .arg("-autoboot_script")
                    .arg(script);
            }
            command
        }
        EmulatorBackend::Fceux => {
            let mut command = Command::new("fceux");
            command
                .arg("--no-config")
                .arg("1")
                .arg("--loadstate")
                .arg("10")
                .arg("--savestate")
                .arg("10")
                .arg("--sound")
                .arg(if run.audio { "1" } else { "0" });
            if let Some(script) = &run.script {
                command.arg("--loadlua").arg(script);
            }
            command.arg(&run.rom);
            command
        }
    };

    if let Some(cwd) = &run.cwd {
        cmd.current_dir(cwd);
    }
    cmd.env("SDL_VIDEODRIVER", "dummy");
    cmd.env("SDL_AUDIODRIVER", "dummy");
    if matches!(run.backend, EmulatorBackend::Fceux) {
        cmd.env("QT_QPA_PLATFORM", "offscreen");
    }
    for (key, value) in &run.env {
        cmd.env(key, value);
    }
    for arg in &run.extra_args {
        cmd.arg(arg);
    }

    if let Some(log_file) = &run.log_file {
        if let Some(parent) = log_file.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create emulator log directory {}",
                    parent.display()
                )
            })?;
        }
        let log = File::create(log_file)
            .with_context(|| format!("failed to create emulator log {}", log_file.display()))?;
        cmd.stdout(Stdio::from(log.try_clone()?));
        cmd.stderr(Stdio::from(log));
    }

    let command_line = format!("{:?}", cmd);
    let started = Instant::now();
    let status = run_with_timeout(&mut cmd, run.timeout_seconds)
        .with_context(|| format!("failed to run emulator command {}", command_line))?;
    let duration_ms = started.elapsed().as_millis();

    if !status.success() {
        bail!(
            "headless emulator run failed (exit {:?}) for {}",
            status.code(),
            command_line
        );
    }

    Ok(HeadlessOutcome {
        backend: run.backend.as_str().to_string(),
        command_line,
        exit_code: status.code(),
        duration_ms,
        log_file: run
            .log_file
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    })
}

fn run_with_timeout(command: &mut Command, timeout_seconds: u64) -> Result<ExitStatus> {
    let mut child = command
        .spawn()
        .context("failed to spawn emulator process")?;
    let deadline = Instant::now() + Duration::from_secs(timeout_seconds);

    loop {
        if let Some(status) = child
            .try_wait()
            .context("failed to poll emulator process")?
        {
            return Ok(status);
        }

        if Instant::now() >= deadline {
            child
                .kill()
                .context("failed to kill timed out emulator process")?;
            let status = child
                .wait()
                .context("failed to wait for timed out emulator process")?;
            bail!(
                "emulator timed out after {} seconds (exit {:?})",
                timeout_seconds,
                status.code()
            );
        }

        thread::sleep(Duration::from_millis(100));
    }
}
