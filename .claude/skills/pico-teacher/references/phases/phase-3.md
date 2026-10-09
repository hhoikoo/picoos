# Phase 3 — A real small OS

**Outcome:** a multitasking kernel with protected user processes, a virtual file system with a RAM file system, a console and a shell that runs programs, and demand-paged virtual memory. The same repo and workspace as Phase 2: everything below builds on it. The system is **device-free**: programs come from a RAM file system whose initial contents are an archive embedded in the kernel image (built by `cargo xtask`); nothing persists across reboots unless the stretch storage work is done.
**Style:** spec-like. Each module lists the expected feature set and tests; internals are the learner's within the pre-decided architecture.

## Tiers (you can stop after any tier)
| Tier | Modules | Points* |
|---|---|---|
| **Core** | Threads and context switching · Preemption, blocking and time · Entering user mode · Program loading and process lifecycle · A safe user–kernel boundary · User runtime · VFS and file descriptors · Console and working directory · Demand paging, stack growth and mmap | 36 |
| **Extended** | Scheduling policy · Copy-on-write and `fork` · Swap and eviction | 12 |
| **Stretch** | Pipes and redirection · job control · device discovery and block driver · block cache and on-disk file system · networking · graphics · SMP · signals · `std` port | open |
\* Sum of module difficulty ratings; a rough proxy, not a time estimate.

## Userland is provided by the teacher
The shell and utilities (`sh`, `ls`, `cat`, `echo`, `pwd`, `mkdir`, later `wc` and others) are ordinary Rust user programs written, built and run by the teacher/agents. The learner writes only the OS side: the syscalls, kernel services and drivers those programs need. The programs double as integration tests of the learner's work. A shell `cd` is a built-in using the `chdir` syscall. The first shell has no pipes or redirection; those arrive with the stretch pipes module.

## Carry-over from Phase 2 (nothing is thrown away)
| Phase 2 piece | What it becomes in Phase 3 |
|---|---|
| The unikernel app (`app_main`) | First runs as a kernel thread (threads module); later the **same source** runs as a ring-3 process (user runtime module). The linked-in unikernel build is retired once the app runs as a process. |
| The small kernel API crate (`console`, `time`, `keyboard`, `alloc`) | The shape of the syscall surface and the user runtime: the user runtime implements the same API on top of syscalls. Its blocking calls (`wait_key`, `sleep_ticks`) become thread/process blocking. |
| Frame allocator + page mapper | Per-process address spaces share the kernel half; reused by demand paging, COW and swap. |
| Kernel heap | Kernel objects (threads, fd tables, VFS nodes) are heap-allocated. |
| Interrupts, timer, keyboard | Preemption, sleeping, console input (line discipline). |
| Kernel-owned GDT and TSS | User segments and a per-process kernel stack are added to the same tables. |
| Higher-half kernel + direct physical map | The kernel half every address space shares; the lower half is for user programs. |
| `cargo xtask`, tests | Same harness; user programs (including the teacher-written userland) are packaged into the kernel image. |

## Pre-decided architecture
- Single CPU first; locks are written to be SMP-correct (atomics, interrupt-disable guards) so SMP stays possible.
- Native syscall ABI via `syscall`/`sysret`: number in `rax`, arguments in `rdi, rsi, rdx, r10, r8, r9`, result in `rax`; the instruction itself clobbers `rcx` and `r11` `[VERIFY]`. Errors are small negative codes mapped to a Rust `Result` in the user runtime.
- Process model: `spawn` (create a new process from an executable), with `fork` as an optional extension in the copy-on-write module.
- Everything a process holds is a file descriptor (typed handles, small integers) backed by a VFS object.
- Kernel thread stacks are allocated through the page mapper with an unmapped guard page below them; size fixed when the module is prepared (default 16 KiB).
- Lock hierarchy documented in code; RAII guards for pinning pages and holding locks across blocking calls.
- Scheduling: round-robin with timer preemption (Core); priority levels and priority inheritance are added by the scheduling policy module (Extended).

---

## Multitasking

### Threads and context switching · difficulty 5 (split into sub-steps)
- **Aims:** run several flows of control on one CPU.
- **Sub-steps:** (a) thread control block and kernel stack with guard page (b) context switch: callee-saved registers and stack swap in a naked function (c) round-robin run queue and `yield` (d) thread exit and reaping (e) idle thread (f) the Phase 2 app runs as a thread.
- **Expected feature set:** create, run, yield and exit threads; a dead thread's resources are freed exactly once; the idle thread runs when nothing else can; no thread runs on a freed stack; stack overflow hits the guard page and is reported; the Phase 2 app runs as a thread (its blocking calls yield to other threads).
- **Look-ups:** System V x86-64 callee-saved registers; Rust `naked` functions (`#[unsafe(naked)]` with `naked_asm!`) `[VERIFY: stability]`; why a new thread's first switch needs a prepared stack.
- **Verification:** host tests for the run queue; QEMU tests: two threads alternate; many create/exit cycles with no leaks; interrupts are never left disabled.
- **Reward:** two threads taking turns on screen.

### Preemption, blocking and time · difficulty 4
- **Aims:** the scheduler becomes real, and time becomes a service.
- **Expected feature set:** timer interrupts preempt running threads; wait queues and blocking; a monotonic clock derived from the timer (and a documented tick rate); `sleep(duration)` built on a timer queue (sleepers wake no earlier than requested and promptly after; zero and negative durations return immediately; many simultaneous sleepers; the CPU idles with `hlt` when all threads sleep); spinlock with an interrupt-disable guard; blocking `Mutex`, semaphore and condition variable; no lost wake-ups.
- **Look-ups:** why a spinlock must disable interrupts on a single CPU; lost-wakeup patterns; PIT vs other time sources `[VERIFY: what QEMU provides]`.
- **Verification:** host tests for the timer queue and wait-queue ordering; QEMU tests: sleep accuracy asserted in ticks with wide tolerance (emulation timing is noisy), producer/consumer, no busy-waiting.
- **Reward:** staggered output as sleepers wake; a bounded-buffer demo.

### Scheduling policy · difficulty 3 · Extended
- **Aims:** decide who runs next.
- **Expected feature set:** priority levels with preemption (a higher-priority thread that becomes runnable preempts immediately), round-robin within a level, `set_priority`; priority inheritance for the blocking mutex so a low-priority holder is boosted while a high-priority thread waits (single and chained); starvation is documented.
- **Stretch:** a multi-level feedback queue with fixed-point load average.
- **Verification:** host tests for run-queue ordering; QEMU tests: classic priority-inversion demo fails without inheritance and passes with it.
- **Reward:** a priority-inversion bug you can watch happen, then fixed.

---

## Processes

### Entering user mode · difficulty 5 (split into sub-steps)
- **Aims:** run untrusted code safely.
- **Sub-steps:** (a) add ring-3 segments to the kernel GDT in the order `sysret` requires (user stack segment then 64-bit user code segment after the base selector; pre-decided when the module is prepared) and set the TSS stack for ring-0 entry (b) a per-process address space whose upper half is the shared kernel (c) jump to ring 3 with a tiny hand-assembled program (d) `syscall`/`sysret` setup via MSRs (`STAR`, `LSTAR`, `SFMASK`), including switching to a kernel stack on entry (`syscall` does not switch stacks) and masking interrupts/direction/trap flags (e) `write` and `exit`.
- **Expected feature set:** a user program runs in ring 3 in its own address space, can print and exit, and cannot touch kernel memory; faults in user mode are caught.
- **Look-ups:** MSR numbers and bit layouts `[VERIFY]`; TSS ring-0 stack usage; segment selector layout for `sysret`; the non-canonical return-address hazard of `sysret`.
- **Verification:** a QEMU test where the user program attempts a privileged instruction and a kernel-address read: the kernel survives and reports.
- **Reward:** a separate program prints and exits.

### Program loading and process lifecycle · difficulty 4
- **Aims:** run real programs, many of them.
- **Expected feature set:** ELF64 loader (segments mapped with correct permissions); arguments (and environment) placed on the user stack; `spawn`, `wait`, `exit`, `getpid`; exit status delivered exactly once; zombies and orphans handled (an init process adopts orphans); a crashing process is terminated without harming others; all resources are released on exit or crash.
- **Look-ups:** ELF64 program headers; stack layout conventions for `argc/argv`.
- **Verification:** user test programs: normal, nested spawn, double wait, wait on a non-child, orphan, crash (null deref, bad jump, illegal instruction), many short-lived processes with no leaks.
- **Reward:** spawn a program that spawns programs.

### A safe user–kernel boundary · difficulty 4
- **Aims:** never trust a pointer.
- **Expected feature set:** typed user-pointer wrappers; `copy_from_user`/`copy_to_user` with page-fault recovery; every syscall validates pointers, lengths and handles; overflow-safe length arithmetic; errors are returned or the offending process is terminated, never a kernel panic.
- **Look-ups:** the fault-recovery technique (exception table or fixup) `[VERIFY]`; integer overflow in range checks.
- **Verification:** an abusive-input test suite (null, kernel addresses, unaligned, wrapping lengths, unmapped middle of a buffer, huge arguments, non-canonical addresses) must leave the kernel running.
- **Reward:** the abuse suite passes.

### User runtime: the same app, now a process · difficulty 3
- **Aims:** close the loop with Phase 2.
- **Expected feature set:** a `user` runtime crate (`_start`, syscall wrappers, panic handler, allocator, `println!`) that implements the Phase 2 kernel-API traits on top of syscalls; the **unchanged Phase 2 app source** builds as a user program and runs. The syscalls it needs exist at this point in simple form: an eager anonymous-memory syscall (replaced by `mmap` later), a raw key-read syscall, `sleep` and a clock. The raw key read stays available alongside the line-disciplined console added later.
- **Look-ups:** how the runtime allocator obtains memory; `#![no_std]` user binaries.
- **Verification:** the Phase 2 app's test passes with the app running as a process.
- **Reward:** the app you wrote in Phase 2 runs in ring 3.
- **Stretch:** port `std` to a native PicoOS target.

---

## Everything is a file

### VFS and file descriptors · difficulty 4
- **Aims:** one interface for files, devices and directories; get a file system working without any disk.
- **Expected feature set:** trait-based file and inode objects; per-process descriptor table with small integer handles; `open`, `close`, `read`, `write`, `seek`, `stat`, `readdir`, `dup`, `create`, `mkdir`, `remove`; path resolution and a mount table; backends: a console device (`/dev/console`) and a writable **RAM file system** whose initial contents come from the archive embedded in the kernel image (initramfs-style); descriptors are released at process exit.
- **Look-ups:** how directory entries and inodes relate; path resolution edge cases (`.`, `..`, trailing slashes, repeated separators).
- **Verification:** host tests for path resolution and the RAM file system; QEMU tests: read an embedded file, create/write/read back, bad descriptor, too many open files, directory listing.
- **Reward:** the provided `cat` and `ls` programs work.

### Console and working directory · difficulty 3
- **Aims:** make the keyboard and screen usable by programs.
- **Expected feature set:** a console line discipline (echo, backspace, line buffering, end-of-file key, blocking reads); per-process current working directory with `chdir` and `getcwd`; relative paths resolve against it; standard input/output/error pre-opened for new processes; spawned processes inherit the working directory.
- **Verification:** scripted keyboard input under QEMU with expected output from the provided shell and utilities (`cd`, `pwd`, `ls`, `cat`, `echo`, `mkdir`).
- **Reward:** the provided shell runs, and you type `cd` and `ls` on your own OS.

---

## Virtual memory

### Demand paging, stack growth and mmap · difficulty 4
- **Aims:** make memory lazy.
- **Expected feature set:** per-process region list (what is mapped where, with permissions); the page-fault handler is the VM entry point; lazy loading of executable segments; zero-fill pages; stack growth within a limit; anonymous and file-backed `mmap`/`munmap` with argument validation and overlap checks (the eager anonymous-memory syscall is retired); no frame leaks at exit.
- **Look-ups:** page-fault error-code bits; designing a region list.
- **Verification:** programs touching huge sparse regions, growing the stack, mapping files; host tests for the region list.
- **Reward:** a program using far more address space than RAM, touching only a little.

### Copy-on-write and `fork` · difficulty 4 · Extended
- **Aims:** share memory safely.
- **Expected feature set:** reference-counted frames; copy-on-write on write fault; `fork` (optional alongside `spawn`) built on COW; shared frames are pinned in the frame table so later eviction never has to update several mappings.
- **Look-ups:** reference counting for frames; TLB invalidation after changing a mapping.
- **Verification:** parent/child isolation tests; frame-count accounting before and after.
- **Reward:** `fork` is nearly free.

### Swap and eviction · difficulty 5 · Extended
- **Aims:** run programs bigger than RAM.
- **Expected feature set:** a `BlockDevice` trait with a RAM-backed implementation holding the swap area (outside the pool of frames the allocator manages; a configurable cap on that pool forces memory pressure); a frame table; clock eviction; per-page state (`Resident | Zero | File | Swap`); pin/unpin as an RAII guard so a page cannot be evicted mid-use; shared (copy-on-write) frames are never evicted; out-of-memory handling; dirty file-backed pages written back on unmap.
- **Look-ups:** the clock (second-chance) algorithm; accessed and dirty bits in page-table entries.
- **Verification:** programs that touch more memory than RAM, concurrently; host tests for the eviction policy.
- **Reward:** a program larger than the (capped) frame pool still runs. With a real disk driver, the swap area can move to disk with no change to the eviction logic.

---

## Stretch (pick any, in any order)

### Pipes and redirection · difficulty 3
Anonymous pipes (blocking, EOF, broken pipe), `dup2`-style redirection, descriptors inherited by `spawn`; the provided shell gains `|`, `>` and `<`. Reward: `ls | wc`. Also: shared-memory regions and message channels.

### Job control
Ctrl-C terminating the foreground process; foreground/background processes.

### Device discovery and a block driver · difficulty 4
PCI configuration-space enumeration; a block driver for one device type chosen when the module is prepared (`virtio-blk` or IDE/AHCI) `[VERIFY: simplest under QEMU]`; sector reads and writes; completion by interrupt or polling, documented. Look-ups: PCI config space and BARs; the device's register/queue layout from its specification. Verification: write a pattern to a second disk image and read it back after restart. Reward: a list of the machine's PCI devices, and a sector read back from disk. PCI discovery and virtio are shared groundwork with networking.

### Block cache and an on-disk file system · difficulty 5
Sub-steps: block cache with write-back and write-behind; on-disk layout (superblock, bitmaps, inode table, direct and indirect blocks); directories; `create`, `unlink`, `mkdir`, `rmdir`, `rename`, link counts; mounted through the VFS using the same `BlockDevice` trait as the RAM-backed device. Expected: files grow and shrink; sparse regions read as zero; safe flushing (including on shutdown); a simple documented crash-consistency story (no journaling); per-inode locking. Verification: host tests for bitmap and inode math; QEMU persistence across restarts. Reward: files survive a reboot, and the system boots its programs from disk.

### Others
Networking (virtio-net driver, ARP/ICMP/UDP, a ping responder); graphics beyond the text console (a framebuffer API, simple drawing, input events, possibly a small game); SMP (APIC, bootstrapping other cores, per-CPU data, IPIs, SMP-safe scheduler); signals; kernel backtraces through frame pointers; ACPI shutdown/reboot; `std` port.

## Explicit non-goals
Users and permissions, security hardening, dynamic linking, a journaling file system, power management, and (by default) persistence across reboots.
