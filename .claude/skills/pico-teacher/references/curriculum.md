# PicoOS curriculum

A from-scratch 64-bit operating system, written entirely in Rust, built by one learner with an AI teacher, in three phases:

1. **Phase 1 — Hello, kernel** (`phases/phase-1.md`). Boot in QEMU using an off-the-shelf bootloader crate; draw text, handle panics, log over serial. Concrete, step-by-step.
2. **Phase 2 — Own the machine, tiny unikernel** (`phases/phase-2.md`). Add interrupts, memory management and a heap, link one app into the kernel image, then replace the bootloader crate with our own boot stages. Mixed guided and spec-style.
3. **Phase 3 — A real small OS** (`phases/phase-3.md`). Threads, scheduling and time; user processes and system calls; a VFS with a RAM file system and console; a shell; demand paging, copy-on-write and swap. Spec-style, built on the same repo as Phase 2. Storage drivers, pipes, networking and graphics are stretch goals.

Each phase has its own file so a session loads only the phase it is working on.

**Forward-only rule:** the project evolves strictly forward. Each phase builds on the code of the previous one and replaces what it supersedes (for example the bootloader crate is deleted when our own boot arrives). There are no compatibility shims, feature flags, dual code paths or kept-alive older builds. Earlier phases are preserved only in git history, as a tagged commit per phase.

**Architecture rule:** 64-bit only (x86-64 long mode). The kernel and every user program are 64-bit. The only 16/32-bit code is the stepping-stone code in the own-boot modules, which exists solely to reach long mode.

**Language rule:** all code we write is Rust. No C, ever. Where hardware forces machine instructions (boot sector, context switch, `hlt`, port I/O), they are written as Rust `asm!` / `global_asm!` / naked functions inside Rust files, kept tiny and explained line by line. Third-party Rust crates are fine for non-assembly logic (fonts, locks, keyboard decoding, boot image building). For single CPU instructions (`in`/`out`, `hlt`, `cli`/`sti`, `lgdt`/`lidt`/`ltr`, control-register moves, `rdmsr`/`wrmsr`, `invlpg`, `iretq`), the learner writes small inline-`asm!` wrappers instead of using an instruction crate, starting in Phase 1.

## Provided infrastructure (done by models, not learner tasks)
Build and tooling are the agents' job. The learner is told briefly how to use them and writes only OS code.
- Toolchain setup: nightly pin, `x86_64-unknown-none` target, QEMU, `llvm-tools`.
- Cargo workspace layout; host-testable logic kept in plain crates so the repo's pre-commit checks (`cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`) keep passing; host-side commands at the workspace root must exclude the `no_std` kernel (for example via `default-members`), and the bare-metal target belongs in `rust-toolchain.toml`.
- `cargo xtask run` / `cargo xtask test`: build kernels and boot stages, assemble disk images, launch QEMU, run kernel tests under a timeout, map `isa-debug-exit` codes to pass/fail.
- Linker scripts, `llvm-objcopy` flat-binary conversion, QEMU flags and trace recipes, debugger attach recipe (tested 2026-10-09 on this Mac: `/usr/bin/lldb`, QEMU 11.1.2): run `qemu-system-x86_64 -drive format=raw,file=<image> -s -S -display none -no-reboot`, then `lldb -b -o "target create <kernel-elf>" -o "gdb-remote localhost:1234" -o "breakpoint set -n _start" -o "continue" -o "register read rip rsp" -o "disassemble -c 5"`. Works from the kernel entry onward in 64-bit mode (source lines, breakpoints, stepping). Caveats: lldb assumes 64-bit, so real/protected-mode code (boot sector, stage 2, 32-bit part) is not debuggable by instruction decoding there (use serial prints and QEMU traces); no Rust language plugin (limited variable inspection); no `gdb` installed and not needed.
- Module skeletons (code with doc comments and `todo!()` bodies), test harnesses, and keeping all of it green.

## Session budget
Goal: Phase 1 in about one session; Phase 2 Parts A-B in two, Part C (own boot) in one or two. To keep token use low:
- The teacher loads only `SKILL.md`, this file's "How this is taught", "Provided infrastructure" and "Session budget", and the **current phase file**. It never re-reads the whole curriculum.
- Each module is prepared in code only (skeleton and tests, no separate documents), just before the module starts, in one batch for the easy modules of a phase.
- Explanations are brief; the learner's look-ups carry the depth. The "predict" step is skipped on difficulty-1/2 modules.
- Progress is read from the repo (code, tests, git history), not from a progress file.
- Easy modules are batched in one go; only difficulty 4-5 modules get sub-step checkpoints.

`[VERIFY]` marks facts not yet checked on this machine (Apple Silicon host, QEMU emulating x86-64). The teacher must check them before teaching them.

---

## 1. How this is taught

### Goals
- **Fun first.** Every module ends with something you can see: a colored screen, a ticking counter, a key echoing back, a crash that is caught instead of rebooting.
- **Hands-on and concrete early, spec-like later.** Phase 1 is a guided walkthrough with exact small steps. Phase 3 hands you a feature set and tests and lets you design internals.
- **Look things up.** You read the docs and hardware descriptions yourself. Agents tell you *what* to look up and *why*, not the answer.
- **Not hair-pulling.** Big design decisions are made for you in advance. Bare-metal failures come with tools and tests.
- **Never fully hand-held, never abandoned.** Hints are a ladder you climb on request.

### What each module contains
Modules in the phase files list the items below that matter for them; the teacher fills in the rest when the module starts:
- **Aims** — what you will understand and build.
- **Concepts** — what the teacher explains before you start.
- **Expected feature set** — the behavior the finished module must have. This is the contract the teacher checks against.
- **Look-ups** — the specific topics you read yourself.
- **Verification** — how correctness is checked (tests and observations).
- **Reward** — the visible result.
- **Difficulty** — 1 (easy) to 5 (hard).

### Module preparation
The curriculum is the only document. Just before a module starts, the teacher prepares it **in code**: a skeleton in the workspace with `todo!()` bodies, exact signatures, doc comments that state the task and any pre-made decisions, and acceptance tests. Concepts, look-up lists and hints are delivered in conversation, not written to files. **There are no reference solutions.** The code you write is checked against the expected feature set and the tests, and reviewed by the teacher.

### How correctness is verified without a reference solution
- **Tests are the contract.** Acceptance tests are written from the module's expected feature set and must include misuse and edge cases, not only the happy path.
- **Author-time validation, not kept.** When preparing a module, the agent proves the tests are fair by running them against a throwaway implementation in a scratch directory. That implementation is deleted afterwards and never shown; the last hint rung is a fresh walkthrough written for the occasion.
- **Review on the fly.** When you finish, the teacher reads your code against the feature set and the invariants for that module (for example "no panic path in the panic handler", "no lock held across an interrupt that takes it") and runs the tests and the QEMU observation.
- **No hallucinated facts.** Hardware details (register bits, descriptor layouts, instruction behavior), crate APIs, QEMU flags and links must be checked against primary sources (vendor manuals, crate docs and source in `~/.cargo/registry`, `qemu-system-x86_64 -help`, the OSDev wiki if reachable) before they are taught. Unverifiable claims are marked `[VERIFY]` or left out. If the teacher is unsure, it says so.

### Hint ladder (one rung at a time, on request)
| Rung | Content |
|---|---|
| 0 | The task and the acceptance tests only. |
| 1 | Which docs/pages to read and the question each answers. |
| 2 | A conceptual nudge: the mechanism in words, no code. |
| 3 | Structure: pseudocode or the shape of the data. |
| 4 | A small code fragment for the single hardest line. |
| 5 | A fuller walkthrough of one approach (used sparingly). |

### Looking up vs. too hard
- **Intended look-ups:** field layouts of descriptors and page-table entries, register bits, a crate's API, QEMU flags, an instruction's behavior, why a test fails.
- **Pre-decided for you:** crates, architecture and module boundaries, data structure choices, lock ordering, syscall numbers and calling conventions, memory layout constants, test harness design.
- **Yours:** implementation details, naming, helper structure, experiments, which stretch goals to take.

### Tests
- **Host unit tests** (`cargo test` on the host) for logic that does not need hardware: glyph placement into a buffer, frame allocator, page-table index math, allocator logic, scheduler queues, fixed-point math, path parsing, bitmaps. Fast; run in the pre-commit hook.
- **Kernel tests under QEMU**, one binary per test: report over serial, finish through QEMU's `isa-debug-exit` with a pass/fail code, with a timeout. Run with `cargo xtask test`.
- **Behavioral tests (phase 3):** small user programs plus expected-output checks covering normal and abusive inputs.
- **Sample code:** short illustrative snippets (for example "how a crate's API is used") are allowed; complete solutions are not.

### Anti-frustration toolkit (built in phase 1, used throughout)
Serial logging, QEMU flags that report CPU resets and exceptions (`-d int,cpu_reset -no-reboot`), a debugger attach recipe (`lldb` against QEMU, tested; see Provided infrastructure), a symbol lookup helper, and a "my kernel reboots in a loop" checklist.

### Honest risks
- Bare-metal bugs fail silently. Mitigation: serial first, tests early, checklist.
- An ARM host emulating x86-64 is slower; `lldb` works only for 64-bit code (tested).
- Descriptor tables and page tables are dense. Mitigation: diagrams, small sub-steps, look-ups, visible results.
- Preparation cost per module (skeleton and tests). Mitigation: just-in-time preparation, batched for easy modules.

---

## 2. Repo layout

```
picoos/
  kernel/      no_std kernel crate (target x86_64-unknown-none)
  kernel-lib/  host-testable pure logic (allocators, queues, math) used by the kernel
  boot/        (phase 2) own boot stages
  app/         (phase 2) the unikernel app crate
  user/        (phase 3) user programs and their runtime crate
  xtask/       host-side: build disk image, run QEMU, run tests
  .claude/skills/pico-teacher/   SKILL.md, references/{curriculum.md, phases/}
```

---

## 3. Phases
- [Phase 1 — Hello, kernel](phases/phase-1.md): 5 modules
- [Phase 2 — Own the machine, tiny unikernel](phases/phase-2.md): kernel services (4), unikernel (1 + 1 stretch), own boot (5)
- [Phase 3 — A real small OS](phases/phase-3.md): Core (threads, preemption and time, user mode, process lifecycle, safe user–kernel boundary, user runtime, VFS, console, demand paging), Extended (scheduling policy, copy-on-write, swap), Stretch (pipes, storage, networking, graphics, SMP, ...)
