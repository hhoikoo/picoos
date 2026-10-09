# Phase 1 — Hello, kernel

**Outcome:** boots in QEMU, logs over serial, writes colored text, shows a panic screen.
**Style:** guided walkthrough. Short concrete steps, exact checkpoints.
**Pre-decided crates (names verified to exist 2026-10-09; the teacher re-checks versions before pinning):** `bootloader` 0.11 (host side), `bootloader_api` 0.11, `noto-sans-mono-bitmap` 0.3 (restrict font sizes/weights via features to keep the kernel small), `spin` `[VERIFY: current version]`. CPU instructions and port I/O are written by the learner as small inline-`asm!` wrappers rather than pulled from crates (see the serial and idle modules).

**Provided by the teacher/agents, not learner tasks:** toolchain setup (target, QEMU), the Cargo workspace (host `cargo test`/`clippy` at the workspace root must not try to run the `no_std` kernel), `cargo xtask run` (builds the kernel, makes the BIOS disk image with `bootloader`, launches QEMU), `cargo xtask test` (runs each kernel test under QEMU with a timeout; maps `isa-debug-exit` codes to pass/fail), the pre-commit setup, debugging recipes, and the **bootloader configuration** used from the first module onward: `entry_point!(kernel_main, config = &CONFIG)` with `mappings.physical_memory` set to map all physical memory at a fixed higher-half offset and the kernel, stack and framebuffer placed in the higher half (`kernel_base`, `dynamic_range_start`), so user space can later own the lower half `[VERIFY: exact fields in bootloader_api 0.11]`. The learner is told how to use these; they are explained briefly, not assigned.

Module count is small on purpose: one focused session should cover the whole phase.

## First boot · difficulty 2
- **Aims:** get code you wrote running on an emulated PC.
- **Concepts:** `#![no_std]`, `#![no_main]`, `#[panic_handler]`, the never type `!`; the bootloader's entry contract (`entry_point!` and what it expands to); a framebuffer: width, height, stride, bytes per pixel, pixel format.
- **Expected feature set:** the kernel entry converts the bootloader crate's `BootInfo` into a **kernel-owned `BootInfo`** in one small adapter function. Its fields: framebuffer description (address, width, height, stride, bytes per pixel, pixel format); memory regions (start, length, usable/reserved); physical-memory offset; kernel image location and size; kernel stack range with an unmapped guard page below it. All other kernel code uses only the kernel-owned type, so a different bootloader can be swapped in later. The kernel fills a rectangle with a color; pixel-offset math lives in a host-testable function.
- **Look-ups:** `BootInfo` and `FrameBufferInfo` in `bootloader_api`.
- **Verification:** host test of offset math (`y * stride + x`, scaled by bytes per pixel); visual check.
- **Reward:** a colored region on screen.

## Serial output · difficulty 2
- **Aims:** a log channel that survives screen bugs, available from the start.
- **Concepts:** port I/O (`in`/`out`) and how to express it with `asm!`; the 16550 UART on COM1 (registers, baud divisor, line control, FIFO, status bit for transmit-ready).
- **Expected feature set:** your own `in`/`out` wrappers in inline `asm!` with a documented safety contract; a minimal UART driver (initialize, write byte, write string); `serial_println!` with format arguments; no screen dependency.
- **Look-ups:** the 16550 register map and initialization sequence; `asm!` operand and option syntax; how QEMU's `-serial stdio` maps COM1 to the terminal.
- **Verification:** kernel output appears in the host terminal; the provided `cargo xtask test` runs a passing and a deliberately failing sample test correctly.
- **Reward:** kernel output in your terminal, and a green/red test run.

## Idle and panics · difficulty 2
- **Aims:** stop burning the CPU; understand panics without an OS.
- **Concepts:** spin loop vs `hlt`; interrupts wake `hlt`; inline `asm!` and its safety contract; why a panic handler is mandatory; `panic = "abort"`; a handler must be total and must not allocate.
- **Expected feature set:** an idle loop built on a documented safe wrapper around `hlt`; a panic handler that logs the panic to serial and halts forever with interrupts disabled, never itself panics, does not allocate.
- **Look-ups:** `asm!` syntax and options (`nomem`, `nostack`, `preserves_flags`); `core::panic::PanicInfo`.
- **Verification:** host CPU usage of QEMU drops while idle (Activity Monitor/`top`); a deliberate `panic!` shows its message on serial and halts cleanly.
- **Reward:** CPU usage drops; panic messages in your terminal.

## Text output · difficulty 3
- **Aims:** draw readable text yourself.
- **Concepts:** pre-rasterized glyphs as intensity maps; blending an intensity into a pixel color; a `Writer` with cursor, colors, newline, wrap and scroll; keeping unsafe pixel writes behind one small boundary; writing the `Writer` against a small surface interface so another display backend can be added later.
- **Expected feature set:** prints ASCII and basic punctuation; handles `\n`; wraps at the right edge; scrolls when full; never writes outside the buffer; shows a visible placeholder for unsupported characters.
- **Look-ups:** `noto-sans-mono-bitmap` API (`get_raster`, `RasterHeight`, `FontWeight`, raster width); pixel formats (RGB/BGR/grayscale) in `FrameBufferInfo`.
- **Verification:** host tests with a fake in-memory framebuffer (wrap, scroll, newline, bounds, placeholder); visual check in QEMU.
- **Reward:** "Hello, PicoOS" on screen.

## `kprintln!` and the panic screen · difficulty 2
- **Aims:** formatted output and a useful panic display.
- **Concepts:** `core::fmt::Write`; a global writer behind `spin::Mutex` and initialization order; declarative macros; avoiding deadlock if a panic occurs while the writer lock is held.
- **Expected feature set:** `kprint!`/`kprintln!` with format arguments; the panic handler additionally clears the screen to a distinct background and prints location and message; it cannot deadlock when the writer is already locked (fallback path).
- **Look-ups:** `macro_rules!` basics; `spin::Mutex::try_lock`; `PanicInfo::location`.
- **Verification:** host test of macro output into a buffer; `panic!("help!")` shows file and line in QEMU and on serial.
- **Reward:** a red panic screen with the location.
