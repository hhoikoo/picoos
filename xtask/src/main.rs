//! Host-side build tooling for PicoOS.
//!
//! ```text
//! cargo xtask build         build the kernel and a bootable BIOS disk image
//! cargo xtask run           build, then boot the image in QEMU
//! cargo xtask check [ARGS]  clippy the kernel for the bare-metal target (ARGS go to cargo clippy)
//! cargo xtask test [NAME]   run kernel tests (kernel/tests/*.rs) under QEMU
//! ```

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use bootloader::BiosBoot;

const KERNEL_TARGET: &str = "x86_64-unknown-none";
const QEMU: &str = "qemu-system-x86_64";
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// QEMU's `isa-debug-exit` device turns a written value `v` into the exit status `(v << 1) | 1`.
const QEMU_EXIT_SUCCESS: i32 = (0x10 << 1) | 1;
const QEMU_EXIT_FAILURE: i32 = (0x11 << 1) | 1;

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("build") => {
            let image = build_image()?;
            println!("{}", image.display());
            Ok(true)
        }
        Some("run") => {
            let image = build_image()?;
            let mut qemu = Command::new(QEMU);
            qemu.args(["-drive", &format!("format=raw,file={}", image.display())]);
            qemu.args(["-serial", "stdio", "-no-reboot"]);
            qemu.args(args);
            Ok(qemu.status().context("failed to start QEMU")?.success())
        }
        Some("check") => check(args.collect()),
        Some("test") => test(args.next().as_deref()),
        _ => {
            eprintln!("usage: cargo xtask <build | run [QEMU args] | check | test [NAME]>");
            Ok(false)
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

fn cargo() -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    command.current_dir(workspace_root());
    command
}

fn run_checked(mut command: Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run {command:?}"))?;
    if !status.success() {
        bail!("{command:?} failed with {status}");
    }
    Ok(())
}

/// Builds the kernel binary and returns the path of the ELF file.
fn build_kernel() -> Result<PathBuf> {
    let mut command = cargo();
    command.args(["build", "--package", "kernel", "--target", KERNEL_TARGET]);
    run_checked(command)?;
    Ok(workspace_root()
        .join("target")
        .join(KERNEL_TARGET)
        .join("debug")
        .join("kernel"))
}

/// Turns a kernel ELF into a bootable BIOS disk image under `target/xtask/`.
fn make_image(kernel: &Path, name: &str) -> Result<PathBuf> {
    let out_dir = workspace_root().join("target").join("xtask");
    fs::create_dir_all(&out_dir)?;
    let image = out_dir.join(format!("{name}.img"));
    BiosBoot::new(kernel)
        .create_disk_image(&image)
        .with_context(|| format!("failed to create a disk image for {}", kernel.display()))?;
    Ok(image)
}

fn build_image() -> Result<PathBuf> {
    let kernel = build_kernel()?;
    make_image(&kernel, "kernel")
}

fn check(extra_args: Vec<String>) -> Result<bool> {
    let mut command = cargo();
    command.args([
        "clippy",
        "--package",
        "kernel",
        "--lib",
        "--bins",
        "--tests",
        "--target",
        KERNEL_TARGET,
    ]);
    command.args(extra_args);
    command.args(["--locked", "--", "-D", "warnings"]);
    run_checked(command)?;
    Ok(true)
}

/// Builds every kernel test binary and returns `(name, path)` pairs.
fn build_tests() -> Result<Vec<(String, PathBuf)>> {
    let mut command = cargo();
    command.args([
        "build",
        "--package",
        "kernel",
        "--tests",
        "--target",
        KERNEL_TARGET,
    ]);
    command.arg("--message-format=json-render-diagnostics");
    command.stdout(Stdio::piped());
    let output = command.output().context("failed to run cargo build")?;
    if !output.status.success() {
        bail!("building the kernel tests failed with {}", output.status);
    }
    let mut tests = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact"
            || !message["target"]["kind"].to_string().contains("test")
        {
            continue;
        }
        if let (Some(name), Some(path)) = (
            message["target"]["name"].as_str(),
            message["executable"].as_str(),
        ) {
            tests.push((name.to_owned(), PathBuf::from(path)));
        }
    }
    tests.sort();
    Ok(tests)
}

fn test(filter: Option<&str>) -> Result<bool> {
    let tests: Vec<_> = build_tests()?
        .into_iter()
        .filter(|(name, _)| filter.is_none_or(|filter| name.contains(filter)))
        .collect();
    if tests.is_empty() {
        println!("no kernel tests found");
        return Ok(true);
    }
    let mut all_passed = true;
    for (name, kernel) in &tests {
        let image = make_image(kernel, name)?;
        let outcome = run_test(&image)?;
        println!("test {name} ... {outcome}");
        all_passed &= outcome == "ok";
    }
    Ok(all_passed)
}

/// Boots one test image headlessly and maps how QEMU ended to a result.
fn run_test(image: &Path) -> Result<&'static str> {
    let mut qemu = Command::new(QEMU);
    qemu.args(["-drive", &format!("format=raw,file={}", image.display())]);
    qemu.args(["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"]);
    qemu.args(["-serial", "stdio", "-display", "none", "-no-reboot"]);
    let mut child = qemu.spawn().context("failed to start QEMU")?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(match status.code() {
                Some(QEMU_EXIT_SUCCESS) => "ok",
                Some(QEMU_EXIT_FAILURE) => "FAILED",
                _ => "FAILED (QEMU ended unexpectedly)",
            });
        }
        if started.elapsed() > TEST_TIMEOUT {
            child.kill()?;
            child.wait()?;
            return Ok("FAILED (timed out)");
        }
        sleep(Duration::from_millis(50));
    }
}
