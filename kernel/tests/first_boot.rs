//! Checks the kernel-owned `BootInfo` against what the machine really looks like, then draws.

#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::arch::asm;
use core::panic::PanicInfo;
use kernel::boot::{framebuffer_buffer, from_bootloader};
use kernel_lib::boot_info::MemoryRegionKind;
use kernel_lib::framebuffer::{Color, Rect, fill_rect, pixel_offset};

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

const EXIT_SUCCESS: u32 = 0x10;
const EXIT_FAILURE: u32 = 0x11;
const HIGHER_HALF: u64 = 0xffff_8000_0000_0000;

fn check(condition: bool) {
    if !condition {
        exit_qemu(EXIT_FAILURE);
    }
}

fn main(boot_info: &'static mut BootInfo) -> ! {
    let info = from_bootloader(boot_info);

    // Memory map: some usable memory, no overlapping nonsense.
    let regions = info.memory_map.as_slice();
    let usable: u64 = regions
        .iter()
        .filter(|r| r.kind == MemoryRegionKind::Usable)
        .map(|r| r.len)
        .sum();
    check(usable > 1024 * 1024);
    check(regions.iter().all(|r| r.start.checked_add(r.len).is_some()));

    // Higher-half layout requested by BOOTLOADER_CONFIG.
    check(info.physical_memory_offset >= HIGHER_HALF);
    check(info.kernel_image.len > 0);
    check(info.kernel_image.virt_base >= HIGHER_HALF);

    // The stack we are running on is the one described, with a guard page below it.
    let here = &raw const info as u64;
    let stack = info.kernel_stack;
    check(stack.len > 0);
    check(stack.bottom >= HIGHER_HALF);
    check((stack.bottom..stack.bottom + stack.len).contains(&here));
    check(stack.guard_page() == Some(stack.bottom - 4096..stack.bottom));

    // Framebuffer: consistent description, and a filled rectangle shows up where it should.
    let Some(fb) = info.framebuffer else {
        exit_qemu(EXIT_FAILURE)
    };
    check(fb.width > 0 && fb.height > 0 && fb.stride >= fb.width);
    check(fb.bytes_per_pixel >= 1);
    check(fb.byte_len >= ((fb.height - 1) * fb.stride + fb.width) * fb.bytes_per_pixel);
    // SAFETY: the bootloader maps `fb.byte_len` bytes at `fb.address`; nothing else uses them.
    let buffer = unsafe { framebuffer_buffer(&fb) };
    check(buffer.len() == fb.byte_len);

    let rect = Rect {
        x: 10,
        y: 10,
        width: 50,
        height: 20,
    };
    let before = pixel_offset(&fb, 9, 10).map(|o| buffer[o]);
    fill_rect(
        buffer,
        &fb,
        rect,
        Color {
            r: 255,
            g: 255,
            b: 255,
        },
    );
    let inside = pixel_offset(&fb, 10, 10).unwrap();
    let outside = pixel_offset(&fb, 9, 10).unwrap();
    check(buffer[inside] == 255);
    check(Some(buffer[outside]) == before);

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
