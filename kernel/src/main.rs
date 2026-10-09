#![no_std]
#![no_main]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::boot::{framebuffer_buffer, from_bootloader};
use kernel_lib::framebuffer::{Color, Rect, fill_rect};

entry_point!(kernel_main, config = &kernel::BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let boot_info = from_bootloader(boot_info);
    let framebuffer_info = boot_info.framebuffer.expect("No frame buffer in machine");

    // SAFETY: `framebuffer_info` was built by `from_bootloader` from the bootloader's own
    // description: the bootloader maps `byte_len` readable and writable bytes at `address`, and
    // keeps them mapped while the kernel runs. This is the only place that touches the
    // framebuffer, so the returned slice is not aliased.
    let framebuffer = unsafe { framebuffer_buffer(&framebuffer_info) };
    let rect = Rect {
        x: 10,
        y: 20,
        width: 100,
        height: 65,
    };
    let color = Color { r: 255, g: 0, b: 0 };
    fill_rect(framebuffer, &framebuffer_info, rect, color);

    loop {
        core::hint::spin_loop()
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
