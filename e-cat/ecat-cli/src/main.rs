// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use clap::{Parser, Subcommand};
use std::fs;
use std::process::{self, Command};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ecat")]
#[command(version, about = "e-cat microservices framework CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new e-cat project
    New {
        /// Project name
        name: String,
    },
    /// Manage protobuf files
    Proto {
        #[command(subcommand)]
        action: ProtoAction,
    },
    /// Run the project in development mode
    Run {
        /// Restart on source changes
        #[arg(long)]
        watch: bool,
    },
    /// Build the project for production
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },
    /// Update all ecat-* workspace dependencies
    Upgrade,
}

#[derive(Subcommand)]
enum ProtoAction {
    /// Add a proto file to the project
    Add {
        /// Path to the proto file
        file: String,
    },
    /// Generate client code from proto
    Client {
        /// Path to the proto file
        file: String,
    },
    /// Generate server code from proto
    Server {
        /// Path to the proto file
        file: String,
        /// Output directory for generated server code
        #[arg(short = 't', long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            if let Err(msg) = ecat_cli::validate_crate_name(&name) {
                eprintln!("Error: invalid project name '{}': {}", name, msg);
                process::exit(1);
            }

            let dir = std::path::Path::new(&name);
            if dir.exists() {
                eprintln!("Error: directory '{}' already exists", name);
                process::exit(1);
            }

            fs::create_dir_all(dir.join("src")).unwrap_or_else(|e| {
                eprintln!("Failed to create project: {}", e);
                process::exit(1);
            });
            fs::create_dir_all(dir.join("proto")).unwrap_or_else(|e| {
                eprintln!("Failed to create proto dir: {}", e);
                process::exit(1);
            });

            let cargo_toml = ecat_cli::generate_cargo_toml(&name);
            fs::write(dir.join("Cargo.toml"), cargo_toml).unwrap_or_else(|e| {
                eprintln!("Failed to write Cargo.toml: {}", e);
                process::exit(1);
            });

            let main_rs = ecat_cli::generate_main_rs();
            fs::write(dir.join("src").join("main.rs"), main_rs).unwrap_or_else(|e| {
                eprintln!("Failed to write main.rs: {}", e);
                process::exit(1);
            });

            let proto_file = ecat_cli::generate_proto_file();
            fs::write(dir.join("proto").join("service.proto"), proto_file).unwrap_or_else(|e| {
                eprintln!("Failed to write service.proto: {}", e);
                process::exit(1);
            });

            println!("Project '{}' created successfully!", name);
            println!();
            println!("  {}/Cargo.toml", name);
            println!("  {}/src/main.rs", name);
            println!("  {}/proto/service.proto", name);
            println!();
            println!("Next steps:");
            println!("  cd {}", name);
            println!("  ecat run");
        }
        Commands::Proto { action } => match action {
            ProtoAction::Add { file } => proto_add(&file),
            ProtoAction::Client { file } => proto_generate(&file, false, None),
            ProtoAction::Server { file, output } => proto_generate(&file, true, output.as_deref()),
        },
        Commands::Run { watch } => {
            if watch {
                run_watch();
            } else {
                run_cargo_run();
            }
        }
        Commands::Build { release } => {
            let mut cmd = Command::new("cargo");
            cmd.arg("build");
            if release {
                println!("Building in release mode...");
                cmd.arg("--release");
            } else {
                println!("Building...");
            }
            let status = cmd.status().unwrap_or_else(|e| {
                eprintln!("Build failed: {}", e);
                process::exit(1);
            });
            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
            println!("Build complete!");
        }
        Commands::Upgrade => upgrade_packages(),
    }
}

fn run_cargo_run() {
    println!("Starting development server...");
    let status = Command::new("cargo")
        .arg("run")
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to start: {}", e);
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn upgrade_packages() {
    let version = env!("CARGO_PKG_VERSION");
    let toml_path = std::path::Path::new("Cargo.toml");
    if !toml_path.exists() {
        eprintln!("Error: no Cargo.toml found in the current directory");
        process::exit(1);
    }
    let content = fs::read_to_string(toml_path).unwrap_or_else(|e| {
        eprintln!("Failed to read Cargo.toml: {}", e);
        process::exit(1);
    });
    let (rewritten, changed) = upgrade_cargo_toml(&content, version);
    if changed == 0 {
        println!("No ecat-* dependencies found in Cargo.toml");
        return;
    }
    fs::write(toml_path, rewritten).unwrap_or_else(|e| {
        eprintln!("Failed to write Cargo.toml: {}", e);
        process::exit(1);
    });
    println!(
        "Updated {} ecat-* dependency requirement(s) to {}",
        changed, version
    );
    let status = Command::new("cargo")
        .arg("update")
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to run cargo update: {}", e);
            process::exit(1);
        });
    if !status.success() {
        eprintln!("cargo update failed; Cargo.lock may be out of date");
        process::exit(status.code().unwrap_or(1));
    }
    println!("Cargo.lock updated");
}

/// Rewrite ecat/ecat-* version requirements in dependency tables.
fn upgrade_cargo_toml(content: &str, version: &str) -> (String, usize) {
    let mut in_deps = false;
    let mut changed = 0;
    let mut out: Vec<String> = Vec::with_capacity(content.lines().count());
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_deps = matches!(
                t,
                "[dependencies]"
                    | "[workspace.dependencies]"
                    | "[dev-dependencies]"
                    | "[build-dependencies]"
            );
            out.push(line.to_string());
            continue;
        }
        if in_deps && let Some(rewritten) = rewrite_ecat_line(line, version) {
            changed += 1;
            out.push(rewritten);
            continue;
        }
        out.push(line.to_string());
    }
    (out.join("\n") + "\n", changed)
}

fn rewrite_ecat_line(line: &str, version: &str) -> Option<String> {
    let eq = line.find('=')?;
    let key = line[..eq].trim();
    if key != "ecat" && !key.starts_with("ecat-") {
        return None;
    }
    let rest = &line[eq + 1..];
    let rest_trimmed = rest.trim();
    if rest_trimmed.starts_with('"') {
        // Plain string requirement: ecat = "1.0"
        let start = line.find('"')?;
        let end = line.rfind('"')?;
        if end <= start || &line[start + 1..end] == version {
            return None;
        }
        let mut s = line.to_string();
        s.replace_range(start + 1..end, version);
        Some(s)
    } else if rest_trimmed.starts_with('{') {
        // Inline table: ecat = { path = "..", version = "1.0" }
        let marker = "version = \"";
        let rel = rest.find(marker)?;
        let val_start = rel + marker.len();
        let rem = &rest[val_start..];
        let val_len = rem.find('"')?;
        let abs_start = eq + 1 + val_start;
        if &line[abs_start..abs_start + val_len] == version {
            return None;
        }
        let mut s = line.to_string();
        s.replace_range(abs_start..abs_start + val_len, version);
        Some(s)
    } else {
        None
    }
}

fn proto_add(file: &str) {
    let path = std::path::Path::new(file);
    if path.exists() {
        eprintln!("Error: proto file '{}' already exists", file);
        process::exit(1);
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Failed to create directory '{}': {}", parent.display(), e);
            process::exit(1);
        });
    }
    let template = format!("syntax = \"proto3\";\n\npackage {};\n", proto_package(file));
    fs::write(path, template).unwrap_or_else(|e| {
        eprintln!("Failed to write '{}': {}", file, e);
        process::exit(1);
    });
    println!("Created proto file: {}", file);
}

/// Infer a proto package name from the file path (parent dirs joined by '.').
fn proto_package(file: &str) -> String {
    let path = std::path::Path::new(file);
    let mut parts: Vec<String> = Vec::new();
    if let Some(parent) = path.parent() {
        for component in parent.components() {
            if let std::path::Component::Normal(seg) = component
                && let Some(seg) = seg.to_str()
            {
                let clean: String = seg
                    .chars()
                    .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                if !clean.is_empty() {
                    parts.push(clean);
                }
            }
        }
    }
    if parts.is_empty() {
        "service".to_string()
    } else {
        parts.join(".")
    }
}

fn proto_generate(file: &str, server: bool, output: Option<&str>) {
    let proto_path = std::path::Path::new(file);
    if !proto_path.exists() {
        eprintln!("Error: proto file '{}' not found", file);
        process::exit(1);
    }
    if !proto_path.starts_with("proto") {
        eprintln!(
            "Error: '{}' is outside the proto/ directory; tonic-build scans proto/ only",
            file
        );
        process::exit(1);
    }
    let kind = if server { "server" } else { "client" };
    let out_dir = match (server, output) {
        (true, Some(out)) => out.to_string(),
        (true, None) => "src/pb_server".to_string(),
        (false, _) => "src/pb_client".to_string(),
    };
    let build_rs = format!(
        r#"// Generated by `ecat proto {kind}`. Do not edit by hand.
fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let protos: Vec<String> = std::fs::read_dir("proto")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "proto"))
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();
    if protos.is_empty() {{
        return Ok(());
    }}
    tonic_build::configure()
        .build_client({client})
        .build_server({server})
        .out_dir("{out_dir}")
        .compile(&protos, &["proto"])?;
    Ok(())
}}
"#,
        client = if server { "false" } else { "true" },
        server = if server { "true" } else { "false" }
    );
    if std::path::Path::new("build.rs").exists() {
        println!("Note: overwriting existing build.rs");
    }
    fs::write("build.rs", build_rs).unwrap_or_else(|e| {
        eprintln!("Failed to write build.rs: {}", e);
        process::exit(1);
    });
    let toml = fs::read_to_string("Cargo.toml").unwrap_or_else(|e| {
        eprintln!("Failed to read Cargo.toml: {}", e);
        process::exit(1);
    });
    let toml = ensure_toml_section(&toml, "build-dependencies", &[("tonic-build", "0.12")]);
    let toml = ensure_toml_section(
        &toml,
        "dependencies",
        &[("tonic", "0.12"), ("prost", "0.13")],
    );
    fs::write("Cargo.toml", toml).unwrap_or_else(|e| {
        eprintln!("Failed to write Cargo.toml: {}", e);
        process::exit(1);
    });
    println!("Generated build.rs (tonic-build {kind} codegen)");
    println!("  proto files: proto/ (scanned at build time)");
    println!("  output:      {out_dir}");
    println!("Updated Cargo.toml: [build-dependencies] tonic-build, [dependencies] tonic + prost");
    println!();
    println!("Next steps:");
    println!("  cargo build  (requires protoc in PATH)");
    println!(
        "  then add `mod pb_{};` to src/main.rs or src/lib.rs",
        if server { "server" } else { "client" }
    );
}

/// Ensure a TOML section lists the given key = "version" dependencies.
fn ensure_toml_section(toml: &str, section: &str, deps: &[(&str, &str)]) -> String {
    let lines: Vec<&str> = toml.lines().collect();
    let mut start: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == format!("[{section}]") {
            start = Some(i);
            break;
        }
    }
    let mut existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    let end = match start {
        None => 0,
        Some(s) => {
            let mut e = lines.len();
            for (i, line) in lines.iter().enumerate().skip(s + 1) {
                let t = line.trim();
                if t.starts_with('[') && t.ends_with(']') {
                    e = i;
                    break;
                }
                if !t.starts_with('#')
                    && let Some(eq) = t.find('=')
                {
                    existing.insert(t[..eq].trim().to_string());
                }
            }
            e
        }
    };
    let missing: Vec<(&str, &str)> = deps
        .iter()
        .copied()
        .filter(|(k, _)| !existing.contains(*k))
        .collect();
    if missing.is_empty() {
        return toml.to_string();
    }
    let insert: Vec<String> = missing
        .iter()
        .map(|(k, v)| format!("{k} = \"{v}\""))
        .collect();
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    match start {
        Some(_) => {
            out.splice(end..end, insert);
        }
        None => {
            if out.last().is_none_or(|l| !l.is_empty()) {
                out.push(String::new());
            }
            out.push(format!("[{section}]"));
            out.extend(insert);
        }
    }
    out.join("\n") + "\n"
}

fn run_watch() {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let relevant = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if relevant {
                    tx.send(()).ok();
                }
            }
        },
        Config::default(),
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to create file watcher: {}", e);
        process::exit(1);
    });
    watcher
        .watch(std::path::Path::new("src"), RecursiveMode::Recursive)
        .unwrap_or_else(|e| {
            eprintln!("Failed to watch src/: {}", e);
            process::exit(1);
        });

    println!("Watching src/ for changes (Ctrl-C to stop)...");
    let mut child = spawn_cargo_run();
    loop {
        if rx.recv().is_err() {
            break;
        }
        // debounce: only restart after 500ms of silence
        while rx.recv_timeout(Duration::from_millis(500)).is_ok() {}
        println!("\nChange detected, restarting...");
        stop_child(&mut child);
        child = spawn_cargo_run();
    }
}

fn spawn_cargo_run() -> std::process::Child {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn().unwrap_or_else(|e| {
        eprintln!("Failed to start: {}", e);
        process::exit(1);
    })
}

/// Stop the `cargo run` child and, on unix, the whole process group it leads,
/// so the spawned service binary does not survive as an orphan holding the port.
fn stop_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        // ESRCH is fine: the child already exited and took its group with it.
        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}
