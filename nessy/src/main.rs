use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use nessy::emulator::{run_headless, EmulatorBackend, HeadlessOutcome, HeadlessRun};
use nessy::ines::inspect_rom;
use nessy::pipeline::{generate_character, generate_roster, sync_runtime_static_includes};
use pulldown_cmark::{html, Options, Parser as MarkdownParser};
use serde::Serialize;
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
    },
    /// Inspect iNES metadata from a ROM.
    Inspect {
        #[arg(long, default_value = "out/smash.nes")]
        rom: PathBuf,
    },
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
        #[arg(last = true)]
        extra_args: Vec<String>,
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

#[derive(Debug)]
struct EmulatorTestCase {
    id: &'static str,
    script: &'static str,
    timeout_seconds: u64,
    description: &'static str,
}

#[derive(Debug, Serialize)]
struct EmulatorCaseResult {
    id: String,
    description: String,
    script: String,
    report_file: String,
    outcome: Option<HeadlessOutcome>,
    status: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct EmulatorSuiteSummary {
    backend: String,
    rom: String,
    passed: usize,
    failed: usize,
    cases: Vec<EmulatorCaseResult>,
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

const EMULATOR_SUITE_CASES: &[EmulatorTestCase] = &[
    EmulatorTestCase {
        id: "boot_smoke",
        script: "boot_smoke.lua",
        timeout_seconds: 20,
        description: "Boots ROM, validates exported signature/state, exits cleanly.",
    },
    EmulatorTestCase {
        id: "frame_progress",
        script: "frame_progress.lua",
        timeout_seconds: 20,
        description: "Verifies frame counter continues increasing under headless execution.",
    },
    EmulatorTestCase {
        id: "input_right",
        script: "input_right.lua",
        timeout_seconds: 20,
        description: "Injects RIGHT input and verifies test position increases.",
    },
    EmulatorTestCase {
        id: "long_stability",
        script: "long_stability.lua",
        timeout_seconds: 35,
        description: "Runs longer session and checks runtime remains responsive.",
    },
];

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = resolve_root(cli.root)?;

    match cli.command {
        TopLevel::Generate(generate) => run_generate(&root, generate.command)?,
        TopLevel::Rom(rom) => run_rom(&root, rom.command)?,
        TopLevel::Emulator(emulator) => run_emulator(&root, emulator.command)?,
        TopLevel::Test(test) => run_test(&root, test.command)?,
        TopLevel::Web(web) => run_web(&root, web.command)?,
        TopLevel::Doctor => run_doctor(),
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
            let sources = ["main.c", "chr.s", "ppubuf_stub.s"];
            let mut objects = Vec::new();

            for source in sources {
                let source_path = source_dir.join(source);
                if !source_path.exists() {
                    bail!("missing required source file: {}", source_path.display());
                }

                let object_path =
                    obj_dir.join(format!("{}.o", source.trim_end_matches(['.', 'c', 's'])));
                let args = vec![
                    "-t".to_string(),
                    "nes".to_string(),
                    "-Oirs".to_string(),
                    "-Cl".to_string(),
                    "-c".to_string(),
                    "-o".to_string(),
                    object_path.to_string_lossy().to_string(),
                    source_path.to_string_lossy().to_string(),
                ];
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

            println!("Built {}", output.display());
        }
        RomSubcommand::Inspect { rom } => {
            let report = inspect_rom(&root.join(rom))?;
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
            extra_args,
        } => {
            let backend: EmulatorBackend = backend.parse()?;
            let run = HeadlessRun {
                backend,
                rom: root.join(rom),
                script: script.map(|value| root.join(value)),
                timeout_seconds,
                cwd: Some(root.to_path_buf()),
                extra_args,
                env: Vec::new(),
            };
            let outcome = run_headless(&run)?;
            println!("{}", serde_json::to_string_pretty(&outcome)?);
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
            let report_dir = root.join(report_dir);
            fs::create_dir_all(&report_dir)
                .with_context(|| format!("failed to create {}", report_dir.display()))?;

            let mut selected = Vec::new();
            for case in EMULATOR_SUITE_CASES {
                if case_filters.is_empty() || case_filters.iter().any(|item| item == case.id) {
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
                let script = suite_dir.join(case.script);
                if !script.exists() {
                    bail!("missing emulator suite script: {}", script.display());
                }

                let case_report = report_dir.join(format!("{}.txt", case.id));
                if case_report.exists() {
                    fs::remove_file(&case_report).with_context(|| {
                        format!("failed to clear previous report {}", case_report.display())
                    })?;
                }

                let run = HeadlessRun {
                    backend,
                    rom: rom_path.clone(),
                    script: Some(script.clone()),
                    timeout_seconds: case.timeout_seconds,
                    cwd: Some(root.to_path_buf()),
                    extra_args: Vec::new(),
                    env: vec![(
                        "NESSY_TEST_OUT".to_string(),
                        case_report.to_string_lossy().to_string(),
                    )],
                };

                match run_headless(&run) {
                    Ok(outcome) => {
                        let (status, detail) = parse_case_report(&case_report)?;
                        if status == "PASS" {
                            passed += 1;
                        } else {
                            failed += 1;
                        }

                        results.push(EmulatorCaseResult {
                            id: case.id.to_string(),
                            description: case.description.to_string(),
                            script: script.to_string_lossy().to_string(),
                            report_file: case_report.to_string_lossy().to_string(),
                            outcome: Some(outcome),
                            status,
                            detail,
                        });
                    }
                    Err(err) => {
                        failed += 1;
                        results.push(EmulatorCaseResult {
                            id: case.id.to_string(),
                            description: case.description.to_string(),
                            script: script.to_string_lossy().to_string(),
                            report_file: case_report.to_string_lossy().to_string(),
                            outcome: None,
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
            if root.join(&snaps).exists() {
                copy_tree(&root.join(snaps), &public_snaps)?;
            } else {
                fs::create_dir_all(&public_snaps)
                    .with_context(|| format!("failed to create {}", public_snaps.display()))?;
            }

            write_build_metadata(root, &public, &public_rom)?;
            println!("Synced web assets into {}", public.display());
        }
    }

    Ok(())
}

fn run_doctor() {
    let checks = [
        ("cargo", true),
        ("cl65", true),
        ("mame", false),
        ("fceux", false),
        ("firefox", false),
        ("dx", false),
    ];

    println!("Toolchain check:\n");
    for (tool, required) in checks {
        let present = binary_on_path(tool);
        let status = if present { "found" } else { "missing" };
        let req = if required { "required" } else { "optional" };
        println!("- {:<8} {:<7} ({})", tool, status, req);
    }
}

fn binary_on_path(program: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(program);
        let candidate_exe = dir.join(format!("{}.exe", program));
        candidate.exists() || candidate_exe.exists()
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
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>docs</title>
    <link rel="stylesheet" href="/main.css" />
  </head>
  <body>
    <main class="page-shell">
      <header class="topbar">
        <a class="brand" href="/">SMASH.NES</a>
        <nav class="topnav">
          <a href="/play">Play</a>
          <a href="/docs">Docs</a>
          <a href="/info">Info</a>
        </nav>
      </header>
      <section class="panel page-copy">
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
"#
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
