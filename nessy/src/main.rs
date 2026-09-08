use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use nessy::audio::{inspect_pcm_wav, WavInspection};
use nessy::emulator::{run_headless, EmulatorBackend, HeadlessOutcome, HeadlessRun};
use nessy::ines::{inspect_rom, set_battery_flag};
use nessy::pipeline::{generate_character, generate_roster, sync_runtime_static_includes};
use nessy::visual::{inspect_screenshot, ScreenshotInspection};
use pulldown_cmark::{html, Options, Parser as MarkdownParser};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(name = "nessy", about = "SmashDotNES build and tooling runner")]
struct Cli {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: TopLevel,
}

#[derive(Debug, Subcommand)]
enum TopLevel {
    /// Generate assets from TOML character specs.
    Generate(GenerateCommand),
    /// Build and inspect ROM artifacts.
    Rom(RomCommand),
    /// Run one headless emulator session.
    Emulator(EmulatorCommand),
    /// Execute named test suites.
    Test(TestCommand),
    /// Sync build outputs into web/public.
    Web(WebCommand),
    /// Validate toolchain availability.
    Doctor,
}

#[derive(Debug, Args)]
struct GenerateCommand {
    #[command(subcommand)]
    command: GenerateSubcommand,
}

#[derive(Debug, Subcommand)]
enum GenerateSubcommand {
    /// Generate one character from a TOML file.
    Character {
        #[arg(long)]
        spec: PathBuf,
    },
    /// Generate all characters discovered under --spec-dir.
    All {
        #[arg(long, default_value = "src/chars")]
        spec_dir: PathBuf,
    },
}

#[derive(Debug, Args)]
struct RomCommand {
    #[command(subcommand)]
    command: RomSubcommand,
}

#[derive(Debug, Subcommand)]
enum RomSubcommand {
    /// Build the NES ROM with cc65.
    Build {
        #[arg(long, default_value = "src/chars")]
        spec_dir: PathBuf,
        #[arg(long, default_value = "src")]
        source_dir: PathBuf,
        #[arg(long, default_value = "src/nessmash.cfg")]
        cfg: PathBuf,
        #[arg(long, default_value = "out/smash.nes")]
        output: PathBuf,
        #[arg(long, default_value = "out/smash.map")]
        map: PathBuf,
        #[arg(long, default_value = "out/smash.lbl")]
        labels: PathBuf,
        #[arg(long)]
        skip_generate: bool,
        #[arg(long, value_enum, default_value_t = TitleVariant::World)]
        title_variant: TitleVariant,
    },
    /// Inspect iNES metadata from a ROM.
    Inspect {
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum TitleVariant {
    World,
    Challengers,
    Emblem,
}

impl TitleVariant {
    fn define_value(self) -> u8 {
        match self {
            Self::World => 0,
            Self::Challengers => 1,
            Self::Emblem => 2,
        }
    }
}

#[derive(Debug, Args)]
struct EmulatorCommand {
    #[command(subcommand)]
    command: EmulatorSubcommand,
}

#[derive(Debug, Subcommand)]
enum EmulatorSubcommand {
    /// Run one headless emulator session.
    Run {
        #[arg(long, default_value = "mame")]
        backend: String,
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
        #[arg(long)]
        script: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        timeout_seconds: u64,
        #[arg(long)]
        audio: bool,
        #[arg(last = true)]
        extra_args: Vec<String>,
    },
    /// Drive named gameplay states and validate native screenshots and optional audio.
    Capture {
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
        #[arg(long, default_value = "nessy/tools/emulator/capture.lua")]
        script: PathBuf,
        #[arg(long, default_value = "out/snaps")]
        output_dir: PathBuf,
        #[arg(long, default_value = "out/reports/emulator/capture.txt")]
        report: PathBuf,
        #[arg(long)]
        audio_output: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        timeout_seconds: u64,
    },
}

#[derive(Debug, Args)]
struct TestCommand {
    #[command(subcommand)]
    command: TestSubcommand,
}

#[derive(Debug, Subcommand)]
enum TestSubcommand {
    /// Run the multi-case emulator suite.
    Emulator {
        #[arg(long, default_value = "mame")]
        backend: String,
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
        #[arg(long, default_value = "nessy/tools/emulator/suite")]
        suite_dir: PathBuf,
        #[arg(long, default_value = "out/reports/emulator")]
        report_dir: PathBuf,
        #[arg(long = "case")]
        case_filters: Vec<String>,
        #[arg(long)]
        fail_fast: bool,
    },
    /// Reject missing, undersized, or visually empty PNG screenshots.
    Screenshots {
        #[arg(long, default_value = "out/reports/web/screens")]
        dir: PathBuf,
    },
}

#[derive(Debug, Args)]
struct WebCommand {
    #[command(subcommand)]
    command: WebSubcommand,
}

#[derive(Debug, Subcommand)]
enum WebSubcommand {
    /// Copy publish assets into web/public and generate web metadata/docs HTML.
    Sync {
        #[arg(long, default_value = "web")]
        web_root: PathBuf,
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
        #[arg(long, default_value = "docs")]
        docs: PathBuf,
        #[arg(long, default_value = "out/snaps")]
        snaps: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
struct EmulatorSuiteManifest {
    backend: String,
    #[serde(rename = "case")]
    cases: Vec<EmulatorTestCase>,
}

#[derive(Debug, Deserialize)]
struct EmulatorTestCase {
    id: String,
    script: String,
    timeout_seconds: u64,
    description: String,
}

#[derive(Debug, Serialize)]
struct EmulatorCaseResult {
    id: String,
    description: String,
    script: String,
    report_file: String,
    outcome: Option<HeadlessOutcome>,
    screenshots: Vec<ScreenshotInspection>,
    status: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct EmulatorSuiteSummary {
    backend: String,
    rom: String,
    manifest: String,
    passed: usize,
    failed: usize,
    cases: Vec<EmulatorCaseResult>,
}

#[derive(Debug, Serialize)]
struct CaptureSummary {
    status: String,
    detail: String,
    outcome: HeadlessOutcome,
    screenshots: Vec<ScreenshotInspection>,
    audio: Option<WavInspection>,
}

#[derive(Debug, Serialize)]
struct BuildLink {
    label: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct GitMetadata {
    branch: Option<String>,
    commit: Option<String>,
    remote: Option<String>,
    dirty: Option<bool>,
}

#[derive(Debug, Serialize)]
struct BuildMetadata {
    project: String,
    generated_at_unix: u64,
    author: Option<String>,
    rom_path: String,
    git: GitMetadata,
    links: Vec<BuildLink>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = resolve_root(cli.root)?;

    match cli.command {
        TopLevel::Generate(generate) => run_generate(&root, generate.command)?,
        TopLevel::Rom(rom) => run_rom(&root, rom.command)?,
        TopLevel::Emulator(emulator) => run_emulator(&root, emulator.command)?,
        TopLevel::Test(test) => run_test(&root, test.command)?,
        TopLevel::Web(web) => run_web(&root, web.command)?,
        TopLevel::Doctor => run_doctor()?,
    }

    Ok(())
}

fn resolve_root(raw: PathBuf) -> Result<PathBuf> {
    if raw.is_absolute() {
        Ok(raw)
    } else {
        Ok(env::current_dir()?.join(raw))
    }
}

fn run_generate(root: &Path, cmd: GenerateSubcommand) -> Result<()> {
    match cmd {
        GenerateSubcommand::Character { spec } => {
            let spec_path = root.join(spec);
            let summary = generate_character(&spec_path, root)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        GenerateSubcommand::All { spec_dir } => {
            let summary = generate_roster(&root.join(spec_dir), root)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
    }
    Ok(())
}

fn run_rom(root: &Path, cmd: RomSubcommand) -> Result<()> {
    match cmd {
        RomSubcommand::Build {
            spec_dir,
            source_dir,
            cfg,
            output,
            map,
            labels,
            skip_generate,
            title_variant,
        } => {
            sync_runtime_static_includes(root)?;

            if !skip_generate {
                let generated = generate_roster(&root.join(spec_dir), root)?;
                println!(
                    "Generated {} characters ({} tiles)",
                    generated.characters.len(),
                    generated.total_tiles
                );
            }

            let out_dir = root.join("out");
            let obj_dir = out_dir.join("obj");
            fs::create_dir_all(&obj_dir)
                .with_context(|| format!("failed to create {}", obj_dir.display()))?;

            let source_dir = root.join(source_dir);
            let sources = ["main.c", "music.c", "chr.s", "ppubuf_stub.s"];
            let mut objects = Vec::new();

            for source in sources {
                let source_path = source_dir.join(source);
                if !source_path.exists() {
                    bail!("missing required source file: {}", source_path.display());
                }

                let object_name = Path::new(source)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .with_context(|| format!("invalid source filename: {source}"))?;
                let object_path = obj_dir.join(format!("{object_name}.o"));
                let mut args = vec![
                    "-t".to_string(),
                    "nes".to_string(),
                    "-Oirs".to_string(),
                    "-Cl".to_string(),
                ];
                if source == "main.c" {
                    args.push("-D".to_string());
                    args.push(format!("TITLE_VARIANT={}", title_variant.define_value()));
                }
                args.extend([
                    "-c".to_string(),
                    "-o".to_string(),
                    object_path.to_string_lossy().to_string(),
                    source_path.to_string_lossy().to_string(),
                ]);
                run_command("cl65", &args, root)?;
                objects.push(object_path);
            }

            let output = root.join(output);
            let map = root.join(map);
            let labels = root.join(labels);
            let cfg = root.join(cfg);

            if !cfg.exists() {
                bail!("missing linker config: {}", cfg.display());
            }
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }

            let mut link_args = vec![
                "-t".to_string(),
                "nes".to_string(),
                "-C".to_string(),
                cfg.to_string_lossy().to_string(),
                "-m".to_string(),
                map.to_string_lossy().to_string(),
                "-Ln".to_string(),
                labels.to_string_lossy().to_string(),
                "-o".to_string(),
                output.to_string_lossy().to_string(),
            ];
            for object in &objects {
                link_args.push(object.to_string_lossy().to_string());
            }
            run_command("cl65", &link_args, root)?;
            set_battery_flag(&output, false)?;

            println!("Built {}", output.display());
        }
        RomSubcommand::Inspect { rom } => {
            let report = inspect_rom(&root.join(rom))?;
            if report.has_battery {
                bail!("ROM unexpectedly declares battery-backed RAM");
            }
            print!("{}", report);
            println!("PASS: iNES checks succeeded");
        }
    }

    Ok(())
}

fn run_emulator(root: &Path, cmd: EmulatorSubcommand) -> Result<()> {
    match cmd {
        EmulatorSubcommand::Run {
            backend,
            rom,
            script,
            timeout_seconds,
            audio,
            extra_args,
        } => {
            let backend: EmulatorBackend = backend.parse()?;
            let run = HeadlessRun {
                backend,
                rom: root.join(rom),
                script: script.map(|value| root.join(value)),
                timeout_seconds,
                audio,
                cwd: Some(root.to_path_buf()),
                extra_args,
                env: Vec::new(),
                log_file: None,
            };
            let outcome = run_headless(&run)?;
            println!("{}", serde_json::to_string_pretty(&outcome)?);
        }
        EmulatorSubcommand::Capture {
            rom,
            script,
            output_dir,
            report,
            audio_output,
            timeout_seconds,
        } => {
            let rom = root.join(rom);
            let script = root.join(script);
            let output_dir = root.join(output_dir);
            let report = root.join(report);
            let audio_output = audio_output.map(|path| root.join(path));
            let runtime_dir = root.join("out/reports/emulator/runtime/capture");
            let cfg_dir = runtime_dir.join("cfg");
            let nvram_dir = runtime_dir.join("nvram");
            fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed to create {}", output_dir.display()))?;
            fs::create_dir_all(&cfg_dir)
                .with_context(|| format!("failed to create {}", cfg_dir.display()))?;
            fs::create_dir_all(&nvram_dir)
                .with_context(|| format!("failed to create {}", nvram_dir.display()))?;
            if let Some(parent) = report.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }

            let expected = [
                "01_title.png",
                "02_lobby.png",
                "03_battle.png",
                "04_walk.png",
                "05_jump.png",
                "06_jab.png",
                "07_special.png",
            ];
            for name in expected {
                let path = output_dir.join(name);
                if path.exists() {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to clear {}", path.display()))?;
                }
            }
            if report.exists() {
                fs::remove_file(&report)
                    .with_context(|| format!("failed to clear {}", report.display()))?;
            }
            if let Some(path) = &audio_output {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                }
                if path.exists() {
                    fs::remove_file(path)
                        .with_context(|| format!("failed to clear {}", path.display()))?;
                }
            }

            let mut extra_args = vec![
                "-cfg_directory".to_string(),
                cfg_dir.to_string_lossy().to_string(),
                "-nvram_directory".to_string(),
                nvram_dir.to_string_lossy().to_string(),
                "-snapshot_directory".to_string(),
                output_dir.to_string_lossy().to_string(),
            ];
            if let Some(path) = &audio_output {
                extra_args.push("-wavwrite".to_string());
                extra_args.push(path.to_string_lossy().to_string());
            }
            let run = HeadlessRun {
                backend: EmulatorBackend::Mame,
                rom,
                script: Some(script),
                timeout_seconds,
                audio: audio_output.is_some(),
                cwd: Some(root.to_path_buf()),
                extra_args,
                env: vec![
                    (
                        "NESSY_CAPTURE_DIR".to_string(),
                        output_dir.to_string_lossy().to_string(),
                    ),
                    (
                        "NESSY_CAPTURE_REPORT".to_string(),
                        report.to_string_lossy().to_string(),
                    ),
                ],
                log_file: Some(root.join("out/reports/emulator/capture.log")),
            };
            let outcome = run_headless(&run)?;
            let (status, detail) = parse_case_report(&report)?;
            if status != "PASS" {
                bail!("capture driver failed: {detail}");
            }
            let screenshots = expected
                .iter()
                .map(|name| inspect_screenshot(&output_dir.join(name)))
                .collect::<Result<Vec<_>>>()?;
            let audio = audio_output.as_deref().map(inspect_pcm_wav).transpose()?;
            let summary = CaptureSummary {
                status,
                detail,
                outcome,
                screenshots,
                audio,
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
    }

    Ok(())
}

fn run_test(root: &Path, cmd: TestSubcommand) -> Result<()> {
    match cmd {
        TestSubcommand::Emulator {
            backend,
            rom,
            suite_dir,
            report_dir,
            case_filters,
            fail_fast,
        } => {
            let backend: EmulatorBackend = backend.parse()?;
            let rom_path = root.join(rom);
            let suite_dir = root.join(suite_dir);
            let manifest_path = suite_dir.join("suite.toml");
            let manifest = load_emulator_suite(&manifest_path)?;
            if manifest.backend != backend.as_str() {
                bail!(
                    "suite {} targets backend '{}', not '{}'",
                    manifest_path.display(),
                    manifest.backend,
                    backend.as_str()
                );
            }
            let report_dir = root.join(report_dir);
            fs::create_dir_all(&report_dir)
                .with_context(|| format!("failed to create {}", report_dir.display()))?;

            let mut selected = Vec::new();
            for case in &manifest.cases {
                if case_filters.is_empty()
                    || case_filters
                        .iter()
                        .any(|item| item.as_str() == case.id.as_str())
                {
                    selected.push(case);
                }
            }
            if selected.is_empty() {
                bail!("no emulator test cases matched filters: {:?}", case_filters);
            }

            let mut results = Vec::new();
            let mut passed = 0usize;
            let mut failed = 0usize;

            for case in selected {
                let script = suite_dir.join(&case.script);
                let case_report = report_dir.join(format!("{}.txt", case.id));
                if !script.exists() {
                    failed += 1;
                    results.push(EmulatorCaseResult {
                        id: case.id.clone(),
                        description: case.description.clone(),
                        script: script.to_string_lossy().to_string(),
                        report_file: case_report.to_string_lossy().to_string(),
                        outcome: None,
                        screenshots: Vec::new(),
                        status: "FAIL".to_string(),
                        detail: format!("missing emulator suite script: {}", script.display()),
                    });
                    if fail_fast {
                        break;
                    }
                    continue;
                }

                if case_report.exists() {
                    fs::remove_file(&case_report).with_context(|| {
                        format!("failed to clear previous report {}", case_report.display())
                    })?;
                }

                let artifact_dir = report_dir.join("artifacts").join(&case.id);
                let runtime_dir = report_dir.join("runtime").join(&case.id);
                let cfg_dir = runtime_dir.join("cfg");
                let nvram_dir = runtime_dir.join("nvram");
                if artifact_dir.exists() {
                    fs::remove_dir_all(&artifact_dir)
                        .with_context(|| format!("failed to reset {}", artifact_dir.display()))?;
                }
                if runtime_dir.exists() {
                    fs::remove_dir_all(&runtime_dir)
                        .with_context(|| format!("failed to reset {}", runtime_dir.display()))?;
                }
                fs::create_dir_all(&artifact_dir)
                    .with_context(|| format!("failed to create {}", artifact_dir.display()))?;
                fs::create_dir_all(&cfg_dir)
                    .with_context(|| format!("failed to create {}", cfg_dir.display()))?;
                fs::create_dir_all(&nvram_dir)
                    .with_context(|| format!("failed to create {}", nvram_dir.display()))?;

                let extra_args = match backend {
                    EmulatorBackend::Mame => vec![
                        "-cfg_directory".to_string(),
                        cfg_dir.to_string_lossy().to_string(),
                        "-nvram_directory".to_string(),
                        nvram_dir.to_string_lossy().to_string(),
                        "-snapshot_directory".to_string(),
                        artifact_dir.to_string_lossy().to_string(),
                    ],
                    EmulatorBackend::Fceux => Vec::new(),
                };

                let run = HeadlessRun {
                    backend,
                    rom: rom_path.clone(),
                    script: Some(script.clone()),
                    timeout_seconds: case.timeout_seconds,
                    audio: false,
                    cwd: Some(root.to_path_buf()),
                    extra_args,
                    env: vec![
                        (
                            "NESSY_TEST_OUT".to_string(),
                            case_report.to_string_lossy().to_string(),
                        ),
                        (
                            "NESSY_ARTIFACT_DIR".to_string(),
                            artifact_dir.to_string_lossy().to_string(),
                        ),
                    ],
                    log_file: Some(report_dir.join("logs").join(format!("{}.log", case.id))),
                };

                match run_headless(&run) {
                    Ok(outcome) => {
                        let (mut status, mut detail) = match parse_case_report(&case_report) {
                            Ok(report) => report,
                            Err(err) => (
                                "FAIL".to_string(),
                                format!("emulator exited without a valid report: {err:#}"),
                            ),
                        };
                        let screenshots = match inspect_screenshot_tree(&artifact_dir) {
                            Ok(screenshots) => screenshots,
                            Err(err) => {
                                status = "FAIL".to_string();
                                detail = format!("{detail}; invalid screenshot artifact: {err:#}");
                                Vec::new()
                            }
                        };
                        if status == "PASS" {
                            passed += 1;
                        } else {
                            failed += 1;
                        }

                        results.push(EmulatorCaseResult {
                            id: case.id.clone(),
                            description: case.description.clone(),
                            script: script.to_string_lossy().to_string(),
                            report_file: case_report.to_string_lossy().to_string(),
                            outcome: Some(outcome),
                            screenshots,
                            status,
                            detail,
                        });
                    }
                    Err(err) => {
                        failed += 1;
                        results.push(EmulatorCaseResult {
                            id: case.id.clone(),
                            description: case.description.clone(),
                            script: script.to_string_lossy().to_string(),
                            report_file: case_report.to_string_lossy().to_string(),
                            outcome: None,
                            screenshots: Vec::new(),
                            status: "FAIL".to_string(),
                            detail: err.to_string(),
                        });
                    }
                }

                if fail_fast && failed > 0 {
                    break;
                }
            }

            let summary = EmulatorSuiteSummary {
                backend: backend.as_str().to_string(),
                rom: rom_path.to_string_lossy().to_string(),
                manifest: manifest_path.to_string_lossy().to_string(),
                passed,
                failed,
                cases: results,
            };

            let summary_path = report_dir.join("summary.json");
            fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)
                .with_context(|| format!("failed to write {}", summary_path.display()))?;

            println!("{}", serde_json::to_string_pretty(&summary)?);
            if failed > 0 {
                bail!("emulator suite failed: {failed} case(s) failed");
            }
        }
        TestSubcommand::Screenshots { dir } => {
            let dir = root.join(dir);
            if !dir.is_dir() {
                bail!("screenshot directory does not exist: {}", dir.display());
            }

            let mut screenshots = WalkDir::new(&dir)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
                })
                .map(|entry| entry.into_path())
                .collect::<Vec<_>>();
            screenshots.sort();
            if screenshots.is_empty() {
                bail!("no PNG screenshots found under {}", dir.display());
            }

            let inspections = screenshots
                .iter()
                .map(|path| inspect_screenshot(path))
                .collect::<Result<Vec<_>>>()?;
            println!("{}", serde_json::to_string_pretty(&inspections)?);
        }
    }

    Ok(())
}

fn parse_case_report(path: &Path) -> Result<(String, String)> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("missing emulator case report {}", path.display()))?;

    let mut lines = raw.lines();
    let status = lines.next().unwrap_or("FAIL").trim().to_string();
    let detail = lines.next().unwrap_or("no detail").trim().to_string();

    Ok((status, detail))
}

fn inspect_screenshot_tree(dir: &Path) -> Result<Vec<ScreenshotInspection>> {
    let mut paths = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    paths.sort();
    paths.iter().map(|path| inspect_screenshot(path)).collect()
}

fn load_emulator_suite(path: &Path) -> Result<EmulatorSuiteManifest> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read emulator suite manifest {}", path.display()))?;
    let mut manifest: EmulatorSuiteManifest = toml::from_str(&raw)
        .with_context(|| format!("invalid emulator suite manifest {}", path.display()))?;
    if manifest.cases.is_empty() {
        bail!("emulator suite manifest has no cases: {}", path.display());
    }

    let mut ids = HashSet::new();
    for case in &manifest.cases {
        if case.id.is_empty()
            || !case
                .id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            bail!("invalid emulator case id '{}': use [A-Za-z0-9_]", case.id);
        }
        if !ids.insert(case.id.clone()) {
            bail!("duplicate emulator case id '{}'", case.id);
        }
        if case.timeout_seconds == 0 {
            bail!("emulator case '{}' has a zero timeout", case.id);
        }
    }
    manifest.cases.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(manifest)
}

fn run_web(root: &Path, cmd: WebSubcommand) -> Result<()> {
    match cmd {
        WebSubcommand::Sync {
            web_root,
            rom,
            docs,
            snaps,
        } => {
            let web_root = root.join(web_root);
            let public = web_root.join("public");
            let public_rom = public.join("rom/smash.nes");
            let public_docs = public.join("docs");
            let public_snaps = public.join("snaps");

            copy_file(&root.join(rom), &public_rom)?;

            if public_docs.exists() {
                fs::remove_dir_all(&public_docs)
                    .with_context(|| format!("failed to remove {}", public_docs.display()))?;
            }
            copy_tree(&root.join(docs), &public_docs)?;
            generate_docs_html(&public_docs)?;

            if public_snaps.exists() {
                fs::remove_dir_all(&public_snaps)
                    .with_context(|| format!("failed to remove {}", public_snaps.display()))?;
            }
            let source_snaps = root.join(snaps);
            if source_snaps.join("01_title.png").is_file() {
                copy_tree(&source_snaps, &public_snaps)?;
            } else {
                fs::create_dir_all(&public_snaps)
                    .with_context(|| format!("failed to create {}", public_snaps.display()))?;
                copy_file(
                    &public.join("assets/title-screen.png"),
                    &public_snaps.join("01_title.png"),
                )?;
            }

            write_build_metadata(root, &public, &public_rom)?;
            println!("Synced web assets into {}", public.display());
        }
    }

    Ok(())
}

fn run_doctor() -> Result<()> {
    let checks = [
        ("cargo", true),
        ("cl65", true),
        ("mame", false),
        ("fceux", false),
        ("node", false),
        ("npm", false),
        ("magick", false),
    ];

    println!("Toolchain check:\n");
    let mut missing_required = Vec::new();
    for (tool, required) in checks {
        let path = binary_on_path(tool);
        let present = path.is_some();
        let status = if present { "found" } else { "missing" };
        let req = if required { "required" } else { "optional" };
        let location = path
            .as_ref()
            .map(|path| format!(" at {}", path.display()))
            .unwrap_or_default();
        println!("- {:<8} {:<7} ({}){}", tool, status, req, location);
        if required && !present {
            missing_required.push(tool);
        }
    }

    if !missing_required.is_empty() {
        bail!(
            "missing required toolchain binaries: {}",
            missing_required.join(", ")
        );
    }
    Ok(())
}

fn binary_on_path(program: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;

    env::split_paths(&paths).find_map(|dir| {
        let candidate = dir.join(program);
        let candidate_exe = dir.join(format!("{}.exe", program));
        if candidate.is_file() {
            Some(candidate)
        } else if candidate_exe.is_file() {
            Some(candidate_exe)
        } else {
            None
        }
    })
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        bail!("source file missing: {}", src.display());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(src, dst)
        .with_context(|| format!("failed to copy {} -> {}", src.display(), dst.display()))?;
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        bail!("source directory missing: {}", src.display());
    }
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src)?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        fs::copy(entry.path(), &target).with_context(|| {
            format!(
                "failed to copy {} -> {}",
                entry.path().display(),
                target.display()
            )
        })?;
    }

    Ok(())
}

fn generate_docs_html(public_docs: &Path) -> Result<()> {
    let mut markdown_files = Vec::new();
    for entry in WalkDir::new(public_docs) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("md") {
            markdown_files.push(entry.path().to_path_buf());
        }
    }
    markdown_files.sort();

    for markdown_path in markdown_files {
        let rel = markdown_path.strip_prefix(public_docs)?;
        let markdown = fs::read_to_string(&markdown_path)
            .with_context(|| format!("failed to read {}", markdown_path.display()))?;
        let title =
            extract_markdown_title(&markdown).unwrap_or_else(|| rel.to_string_lossy().to_string());
        let body_html = markdown_to_html(&markdown);
        let page_html = render_doc_html_page(&title, &body_html, rel);
        let html_path = public_docs.join(rel).with_extension("html");

        if let Some(parent) = html_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&html_path, &page_html)
            .with_context(|| format!("failed to write {}", html_path.display()))?;

        write_clean_doc_route(public_docs, rel, &page_html)?;
    }

    Ok(())
}

fn write_clean_doc_route(public_docs: &Path, markdown_rel: &Path, page_html: &str) -> Result<()> {
    let route_rel = markdown_route_rel(markdown_rel);
    if route_rel.as_os_str().is_empty() {
        return Ok(());
    }

    let route_index = public_docs.join(route_rel).join("index.html");
    if let Some(parent) = route_index.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&route_index, page_html)
        .with_context(|| format!("failed to write {}", route_index.display()))?;
    Ok(())
}

fn markdown_route_rel(markdown_rel: &Path) -> PathBuf {
    // Example: `implementation_notes.md` -> `implementation-notes/`
    // and `guides/foo.md` -> `guides/foo/` for clean `/docs/...` routes.
    let mut out = PathBuf::new();
    if let Some(parent) = markdown_rel.parent() {
        for comp in parent.components() {
            if let std::path::Component::Normal(seg) = comp {
                if let Some(raw) = seg.to_str() {
                    out.push(slug_segment(raw));
                }
            }
        }
    }

    if let Some(stem) = markdown_rel.file_stem().and_then(|v| v.to_str()) {
        if !stem.eq_ignore_ascii_case("readme") {
            out.push(slug_segment(stem));
        }
    }

    out
}

fn slug_segment(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "doc".to_string()
    } else {
        trimmed.to_string()
    }
}

fn extract_markdown_title(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("# ")
            .map(|value| value.trim().to_string())
    })
}

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MarkdownParser::new_ext(markdown, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn render_doc_html_page(title: &str, body_html: &str, markdown_rel_path: &Path) -> String {
    let title = escape_html(title);
    let markdown_rel_display = markdown_rel_path.to_string_lossy().replace('\\', "/");
    let markdown_source_url = format!("/docs/{markdown_rel_display}");
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="theme-color" content="#070808" />
    <meta name="color-scheme" content="dark" />
    <meta name="description" content="Smash.NES technical manual: {title}." />
    <title>{title} — Smash.NES Manual</title>
    <link rel="icon" href="data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 32 32%22><rect width=%2232%22 height=%2232%22 rx=%224%22 fill=%22%23070808%22/><path d=%22M7 9h18v4H11v3h12v4H11v3h14v4H7z%22 fill=%22%23e64b31%22/></svg>" />
    <link rel="stylesheet" href="/main.css" />
  </head>
  <body>
    <a class="skip-link" href="#main">Skip to manual</a>
    <main id="main" class="page-shell">
      <header class="topbar">
        <a class="brand" href="/" aria-label="Smash.NES home">
          <span class="power-led" aria-hidden="true"></span>
          <span>SMASH.NES</span>
        </a>
        <nav class="topnav" aria-label="Primary navigation">
          <a href="/play/">Play</a>
          <a href="/docs/" aria-current="page">Manual</a>
          <a href="/info/">Build</a>
        </nav>
      </header>
      <section class="panel page-copy">
        <p class="eyebrow">Technical manual</p>
        <h1 class="page-title">{title}</h1>
        <p class="muted doc-meta">source: {markdown_rel_display}</p>
        <article class="doc-article">
{body_html}
        </article>
        <p class="muted">Source markdown: <a href="{markdown_source_url}">{markdown_rel_display}</a></p>
      </section>
    </main>
  </body>
</html>
"##
    )
}

fn write_build_metadata(root: &Path, public: &Path, public_rom: &Path) -> Result<()> {
    let info_dir = public.join("info");
    fs::create_dir_all(&info_dir)
        .with_context(|| format!("failed to create {}", info_dir.display()))?;

    let build = collect_build_metadata(root, public, public_rom);
    let out = serde_json::to_string_pretty(&build)?;
    let info_path = info_dir.join("build.json");
    fs::write(&info_path, out)
        .with_context(|| format!("failed to write {}", info_path.display()))?;
    Ok(())
}

fn collect_build_metadata(root: &Path, public: &Path, public_rom: &Path) -> BuildMetadata {
    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let author = env::var("GIT_AUTHOR_NAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| capture_command_output(root, "git", &["config", "--get", "user.name"]));

    let git = GitMetadata {
        branch: capture_command_output(root, "git", &["rev-parse", "--abbrev-ref", "HEAD"]),
        commit: capture_command_output(root, "git", &["rev-parse", "--short", "HEAD"]),
        remote: capture_command_output(root, "git", &["remote", "get-url", "origin"]),
        dirty: git_dirty_status(root),
    };

    let rom_path = public_rom
        .strip_prefix(public)
        .ok()
        .map(|rel| format!("/{}", rel.to_string_lossy().replace('\\', "/")))
        .unwrap_or_else(|| "/rom/smash.nes".to_string());

    let links = vec![
        BuildLink {
            label: "Home".to_string(),
            url: "/".to_string(),
        },
        BuildLink {
            label: "Play".to_string(),
            url: "/play".to_string(),
        },
        BuildLink {
            label: "Docs".to_string(),
            url: "/docs".to_string(),
        },
        BuildLink {
            label: "Info".to_string(),
            url: "/info".to_string(),
        },
        BuildLink {
            label: "ROM".to_string(),
            url: rom_path.clone(),
        },
    ];

    BuildMetadata {
        project: "SmashDotNES".to_string(),
        generated_at_unix,
        author,
        rom_path,
        git,
        links,
    }
}

fn capture_command_output(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn git_dirty_status(cwd: &Path) -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(!text.trim().is_empty())
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn run_command(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    println!("$ {} {}", program, args.join(" "));

    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to launch {}", program))?;

    if !status.success() {
        bail!("command failed: {} {}", program, args.join(" "));
    }

    Ok(())
}
