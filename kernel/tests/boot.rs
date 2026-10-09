//! Boot smoke test: the kernel boots under QEMU and reports success through the debug-exit device.

#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::arch::asm;
use core::panic::PanicInfo;

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

/// Values written to QEMU's `isa-debug-exit` device (see `xtask`); QEMU's exit status is `(value << 1) | 1`.
const EXIT_SUCCESS: u32 = 0x10;
const EXIT_FAILURE: u32 = 0x11;

fn main(_boot_info: &'static mut BootInfo) -> ! {
    exit_qemu(EXIT_SUCCESS)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(EXIT_FAILURE)
}

fn exit_qemu(code: u32) -> ! {
    // SAFETY: port 0xf4 is the `isa-debug-exit` device configured by `cargo xtask test`;
    // writing to it ends the virtual machine and touches no memory.
    unsafe {
        asm!("out dx, eax", in("dx") 0xf4_u16, in("eax") code, options(nomem, nostack, preserves_flags));
    }
    loop {
        core::hint::spin_loop();
    }
}
