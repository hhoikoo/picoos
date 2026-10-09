# Phase 2 — Own the machine (tiny unikernel)

**Outcome:** the kernel handles interrupts, manages memory and a heap; one app crate is linked into the single image and runs on kernel services (console, timer, keyboard, heap); finally our own boot stages replace the bootloader crate and switch the CPU to 64-bit mode. No user mode, no processes.
**Style:** mixed. Boot modules are guided (dense and unforgiving); kernel modules are more spec-like.

**Provided by the teacher/agents, not learner tasks:** linker scripts, packaging boot stages and the kernel into a disk image (`cargo xtask`), flat-binary conversion (`llvm-objcopy`), QEMU invocations (including trace flags `-d int,cpu_reset -no-reboot`), the test harness, and debug recipes. The learner writes only OS code.

## Carry-over from Phase 1
| Phase 1 piece | Role in Phase 2 |
|---|---|
| Kernel-owned `BootInfo` + adapter | The target the own-boot stages must produce (boot information module) |
| `Writer` over a surface interface, `kprintln!`, panic screen | Carried forward as is; they only depend on the kernel-owned `BootInfo` |
| Serial output | Also used from the boot stages (port I/O works in real mode), since the debugger cannot decode 16/32-bit code |
| `cargo xtask run/test`, QEMU harness | Switches to images built from our own boot stages; the same tests keep running |

Order: kernel services and the unikernel first (quick, visible rewards, tests in place), then own the boot as the finale. Own boot is a modular finale: Phase 3 can start without it. Expect about two sessions for Parts A–B and one or two for Part C.

## Part A — Kernel services (on the bootloader crate)

### Exceptions · difficulty 4 (two sub-steps)
- **Aims:** make CPU faults visible and survivable.
- **Sub-steps:** (a) kernel-owned IDT with a breakpoint handler that resumes; (b) kernel-owned GDT (kernel code, kernel data, TSS) loaded at init, an IST stack, and the double-fault handler. The kernel never relies on a GDT left behind by a bootloader.
- **Concepts:** exceptions vs interrupts; IDT and gate descriptors; the interrupt stack frame; error codes; breakpoint and double fault; the IST for a safe double-fault stack; handler ABI options (`extern "x86-interrupt"` vs naked stub) `[VERIFY: current stability and soundness notes]`.
- **Expected feature set:** an IDT with handlers for divide error, breakpoint, invalid opcode, general protection, page fault and double fault; each prints a diagnostic (name, error code, instruction pointer, `CR2` for page faults); the breakpoint resumes execution.
- **Look-ups:** IDT gate layout; which exceptions push an error code; TSS and IST setup. Pre-decided: the double-fault IST stack and the kernel boot stack have unmapped guard pages (the boot stack's guard page comes from the boot contract).
- **Verification:** QEMU tests trigger `int3` and a divide-by-zero; a stack-overflow test expects a double-fault report, not a reboot.
- **Reward:** a stack overflow reports "double fault" instead of rebooting.

### Hardware interrupts · difficulty 3
- **Aims:** react to timers and keys.
- **Concepts:** 8259 PIC remap and end-of-interrupt; PIT; PS/2 keyboard scancodes; enabling interrupts (`sti`); never hold a lock across an interrupt that takes it. `[VERIFY: PIC vs APIC under this boot path in QEMU]`
- **Expected feature set:** timer ticks at a known frequency; keyboard input decoded to characters with the `pc-keyboard` crate `[VERIFY: current version]`; both handlers do minimal work and send EOI.
- **Look-ups:** PIC remap sequence (ICW1–4); PIT channel 0 programming; scancode set 1 vs 2.
- **Verification:** tick counter increases at the expected rate (with tolerance); scripted keystrokes via the QEMU monitor `[VERIFY]`.
- **Reward:** a ticking clock and typed characters on screen.

### Physical memory and paging · difficulty 4
- **Aims:** manage RAM and virtual address spaces.
- **Concepts:** the boot memory map to usable frames; a frame allocator; 4-level page tables; the TLB and `invlpg`; page faults; the kernel's address-space layout.
- **Expected feature set:** (the bootloader crate is configured to map all physical memory at a fixed offset and place the kernel in the higher half; this layout is the contract the own-boot finale must reproduce) a frame allocator that never hands out kernel, boot or reserved frames; `map`, `unmap` and `translate` for 4 KiB pages; page faults report the faulting address and flags.
- **Look-ups:** page-table entry bits (present, writable, user, NX, accessed, dirty); `CR3`; how the kernel reaches page tables (identity or offset mapping).
- **Verification:** host tests for the allocator and index math; QEMU test maps a fresh frame, writes, reads, unmaps (a later read faults).
- **Reward:** write and read through a page you mapped yourself.

### Kernel heap · difficulty 3
- **Aims:** get `Vec`, `String` and `Box` in the kernel.
- **Concepts:** `#[global_allocator]`; bump allocator, then fixed-size blocks; fragmentation; locking in an allocator.
- **Expected feature set:** a heap region mapped with the paging module; an allocator that handles alignment, reuses freed memory, and reports out-of-memory without undefined behavior.
- **Look-ups:** the `GlobalAlloc` contract; `Layout`.
- **Verification:** host tests over a plain byte buffer; QEMU stress test (many small and mixed sizes, short- and long-lived).
- **Reward:** `Vec` and `String` work in your kernel.

## Part B — The unikernel

### The unikernel boundary · difficulty 2
- **Aims:** separate "the app" from "the kernel" inside one image.
- **Concepts:** one image, one address space, no privilege split; a small kernel API crate (`console`, `time`, `keyboard`, `alloc`); the app entry `fn app_main()`.
- **Expected feature set:** an `app` crate that depends only on a kernel API crate defined as traits/functions (`console`, `time`, `keyboard`, `alloc`) that could be reimplemented on top of system calls later (Phase 3 runs this same app as a user process), and uses `#![forbid(unsafe_code)]`; the kernel initializes everything, then calls `app_main`; the API offers blocking calls (`wait_key`, `sleep_ticks`) rather than polling so the app can later run as a thread or process without starving others; the app prints a greeting, shows a tick counter and echoes typed keys.
- **Look-ups:** workspace dependency structure; `#![forbid(unsafe_code)]`.
- **Verification:** QEMU test with scripted keystrokes; the app crate compiles with unsafe forbidden.
- **Reward:** your app running on your OS.

### (stretch) Async executor · difficulty 4
- **Aims:** concurrency without threads.
- **Concepts:** `Future`, `Waker`, a simple executor driven by timer and keyboard events.
- **Expected feature set:** tasks that await a timer and a key; a clock task and a keyboard task running concurrently.
- **Verification:** QEMU test with deterministic ordering.
- **Reward:** two things happening at once.

## Part C — Own the boot (finale)

With a working kernel and a clear target (the kernel-owned `BootInfo` plus what the kernel already needs from boot), replace the bootloader crate with our own stages. Boot stages also log to the serial port; the debugger cannot decode 16/32-bit code.

**Pre-decided design:** stage 1 and stage 2 are hand-written 16/32-bit code in `global_asm!` (small subroutines for disk read, E820, A20, VBE); the kernel is linked non-PIE at a fixed higher-half address (`relocation-model=static`, `code-model=kernel` `[VERIFY]`) and packaged by `cargo xtask` as a flat binary, so the stages copy it rather than parse ELF. Ownership: the long mode module builds the page tables (identity alias for the jump, higher-half kernel, direct map of physical memory); the boot information module passes `BootInfo`, and the kernel drops the identity alias in its own init after entry.

### Boot sector · difficulty 3
- **Aims:** run the first instructions you assembled.
- **Concepts:** 16-bit real mode and `segment:offset` addressing; the BIOS loads sector 0 at `0x7C00`; the `0xAA55` signature; BIOS teletype output (`int 0x10`).
- **Expected feature set:** a 512-byte boot image, written as Rust `global_asm!` 16-bit code, that prints a message via BIOS and halts.
- **Look-ups:** boot signature rules; `int 0x10` function `0x0E`; `.code16` in `global_asm!` `[VERIFY]`.
- **Verification:** boots in QEMU; hexdump shows `55 AA` at bytes 510–511; size exactly 512 bytes.
- **Reward:** your message printed by your own bootloader.

### Stage 2: load, probe and set a video mode · difficulty 4 (sub-steps: load, E820, A20, VBE)
- **Aims:** escape the 512-byte limit and ask the BIOS what it knows before it goes away.
- **Concepts:** BIOS disk services (`int 0x13` extended read); boot drive in `dl`; a stack; BIOS disk reads land in low memory, so the kernel is loaded low in chunks and copied up once in protected mode `[VERIFY]` (conventional memory is under about 610 KiB, so the kernel image must stay small: restrict font sizes); the E820 memory map (`int 0x15, EAX=0xE820`); the A20 line; VBE (VESA BIOS Extensions) to get a linear pixel framebuffer while BIOS services still exist.
- **Expected feature set:** stage 1 loads stage 2 (more than 512 bytes) and jumps to it; stage 2 loads the kernel image into low memory, stores the E820 entries at a fixed address in a documented format, prints only the region count (the Rust kernel prints the table later over serial), enables and verifies A20, and as its last real-mode action sets a linear-framebuffer VBE mode (32 bpp preferred) and records its address, pitch, size and pixel layout in the boot information.
- **Look-ups:** disk address packet layout (`int 0x13 AH=0x42`); E820 entry layout and region types; A20 enable methods and a test; VBE functions `0x4F00`/`0x4F01`/`0x4F02` (controller info, mode info, set mode with the linear-framebuffer bit) `[VERIFY: supported by QEMU's default VGA under SeaBIOS]`.
- **Verification:** printed regions match the RAM given to QEMU (`-m`); a test image where stage 2 is larger than 512 bytes.
- **Reward:** the region count printed at boot, then the screen flips to pixel mode.

### Protected mode · difficulty 4
- **Aims:** cross from 16-bit to 32-bit mode.
- **Concepts:** GDT and segment descriptors; `CR0.PE`; the far jump; why BIOS services stop working; writing pixels to the VBE framebuffer and text to the serial port (the `0xb8000` text buffer is no longer shown once a graphics mode is set).
- **Expected feature set:** a flat 4 GiB GDT (code and data); stage 2 enters protected mode and 32-bit code draws to the framebuffer and logs over serial without BIOS help; the kernel image is copied above 1 MiB.
- **Look-ups:** segment descriptor bit layout (base, limit, access byte, flags); the entry sequence.
- **Verification:** pixels and serial text from 32-bit code; the trace flags show no resets.
- **Reward:** pixels drawn without BIOS help.

### Long mode · difficulty 5 (split into sub-steps)
- **Aims:** reach 64-bit mode.
- **Concepts:** 4-level paging; PAE (`CR4.PAE`); `EFER.LME`; `CR0.PG`; the 64-bit code segment; far jump into 64-bit code.
- **Sub-steps:** (a) identity-mapped page tables (2 MiB pages suffice) (b) enable PAE, load CR3 (c) set `EFER.LME` via `rdmsr`/`wrmsr` (d) enable paging (e) load a 64-bit GDT and far-jump (f) call a Rust `extern "C"` entry.
- **Expected feature set:** 64-bit Rust code runs from the loaded kernel with a stack and the memory map passed in. Page tables map the kernel in the higher half (and keep a temporary identity alias for the jump), plus a direct map of physical memory at a fixed offset; user programs in Phase 3 will use the lower half.
- **Look-ups:** page-table entry bits; MSR `0xC0000080`; the long-mode code-segment descriptor flag.
- **Verification:** the Rust entry prints over serial and draws to the framebuffer; triple-fault diagnosis recipe ready beforehand.
- **Reward:** Rust code runs on a machine you booted yourself.

### Boot information and the hand-off · difficulty 3
- **Aims:** give the kernel what the off-the-shelf bootloader used to.
- **Concepts:** defining our own boot-info structure (memory map, display description, kernel size) with a stable `#[repr(C)]` layout, describing the VBE framebuffer in the same terms Phase 1's writer already uses.
- **Expected feature set:** the own-boot stages fill the kernel-owned `BootInfo` fields the kernel already consumes: the E820-derived memory regions, the VBE framebuffer description, the physical-memory offset (all physical memory mapped at a fixed offset), the kernel image location and size, the kernel stack range with an unmapped guard page below it, with the kernel linked and mapped in the higher half (an identity alias is dropped once running). The own boot path replaces the `bootloader` crate: the crate, its configuration and the adapter that converted its `BootInfo` are deleted, and `cargo xtask` builds images from our stages only.
- **Look-ups:** `#[repr(C)]` rules; how the framebuffer description in the boot information maps to Phase 1's surface interface.
- **Verification:** all Phase 1 and Phase 2 kernel tests pass on the new boot path (paging and the heap depend on the offset and the memory map); the same panic screen appears on the pixel framebuffer.
- **Reward:** your whole kernel, now on your own bootloader.

## Stretch: UEFI boot path
A second own-boot path as a Rust UEFI application (`x86_64-unknown-uefi`): firmware already provides 64-bit mode and a graphics framebuffer (GOP), so the real/protected/long-mode work disappears and the new learning is UEFI boot services (memory map, `ExitBootServices`). Same boot-information structure as the own-boot path. Optional; not required for any later module.
