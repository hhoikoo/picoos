# PicoOS

A from-scratch 64-bit operating system in Rust.

## Status

Boots to a blank screen under QEMU; the kernel itself is just starting. The roadmap is in [`.claude/skills/pico-teacher/references/curriculum.md`](.claude/skills/pico-teacher/references/curriculum.md).

Planned:

1. Boot in QEMU, draw text, handle panics, log over serial.
2. Interrupts, memory management and a heap, with one app linked into the kernel image.
3. Threads, user programs, virtual memory, a file system and a shell.

## Requirements

- Rust nightly with the `x86_64-unknown-none` target (installed from `rust-toolchain.toml`)
- [QEMU](https://www.qemu.org/) (`qemu-system-x86_64`)

## Run

```sh
cargo xtask run     # build the kernel and boot it in QEMU
cargo xtask test    # run the kernel tests under QEMU
cargo xtask check   # clippy the kernel for the bare-metal target
```

## Develop

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo xtask check
```

The first `cargo test` installs a pre-commit hook (via [husky-rs](https://crates.io/crates/husky-rs)) that runs the checks above.

## Claude Code

The repo includes a skill, `pico-teacher`, for working through the roadmap.

## License

[Apache-2.0](LICENSE)
