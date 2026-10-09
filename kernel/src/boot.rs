//! The one place that knows about `bootloader_api`: it converts the bootloader's `BootInfo`
//! into the kernel's own (`kernel_lib::boot_info::BootInfo`).

use kernel_lib::boot_info::{
    BootInfo, FramebufferInfo, KernelImage, KernelStack, MemoryMap, MemoryRegion, MemoryRegionKind,
};

/// Converts what `bootloader_api` hands the kernel into the kernel-owned [`BootInfo`].
///
/// * Memory regions: `Usable` stays `Usable`, every other kind becomes `Reserved`. Convert
///   `start`/`end` into `start`/`len`. Panic with a clear message if there are more than
///   `MAX_MEMORY_REGIONS`.
/// * Framebuffer: `None` if the bootloader gave none. Its `address` is the virtual address of
///   the first byte. Map `PixelFormat::{Rgb, Bgr, U8}` to `{Rgb, Bgr, Gray}`; everything else
///   is `Unknown`.
/// * `physical_memory_offset` is required (our `BOOTLOADER_CONFIG` asks for it): panic with a
///   clear message if it is missing.
/// * The kernel image and stack fields map to `kernel_addr`/`kernel_image_offset`/`kernel_len`
///   and `kernel_stack_bottom`/`kernel_stack_len`.
pub fn from_bootloader(info: &bootloader_api::BootInfo) -> BootInfo {
    let framebuffer: Option<FramebufferInfo> = info.framebuffer.as_ref().map(into_framebuffer_info);
    let memory_map = into_memory_map(&info.memory_regions);
    let physical_memory_offset = info
        .physical_memory_offset
        .into_option()
        .expect("Missing required value physical_memory_offset");
    let kernel_image = KernelImage {
        phys_addr: info.kernel_addr,
        virt_base: info.kernel_image_offset,
        len: info.kernel_len,
    };
    let kernel_stack = KernelStack {
        bottom: info.kernel_stack_bottom,
        len: info.kernel_stack_len,
    };

    BootInfo {
        framebuffer,
        memory_map,
        physical_memory_offset,
        kernel_image,
        kernel_stack,
    }
}

fn into_framebuffer_info(frame_buffer: &bootloader_api::info::FrameBuffer) -> FramebufferInfo {
    let info = frame_buffer.info();
    let pixel_format = match info.pixel_format {
        bootloader_api::info::PixelFormat::Rgb => kernel_lib::boot_info::PixelFormat::Rgb,
        bootloader_api::info::PixelFormat::Bgr => kernel_lib::boot_info::PixelFormat::Bgr,
        bootloader_api::info::PixelFormat::U8 => kernel_lib::boot_info::PixelFormat::Gray,
        _ => kernel_lib::boot_info::PixelFormat::Unknown,
    };

    FramebufferInfo {
        address: frame_buffer.buffer().as_ptr() as u64,
        byte_len: info.byte_len,
        width: info.width,
        height: info.height,
        stride: info.stride,
        bytes_per_pixel: info.bytes_per_pixel,
        pixel_format,
    }
}

fn into_memory_map(memory_regions: &bootloader_api::info::MemoryRegions) -> MemoryMap {
    let mut memory_map = MemoryMap::new();

    for memory_region in memory_regions.iter() {
        let len = memory_region
            .end
            .checked_sub(memory_region.start)
            .expect("invalid memory region: attempt to subtract with underflow");
        let memory_region = MemoryRegion {
            start: memory_region.start,
            len,
            kind: match memory_region.kind {
                bootloader_api::info::MemoryRegionKind::Usable => MemoryRegionKind::Usable,
                _ => MemoryRegionKind::Reserved,
            },
        };

        memory_map
            .push(memory_region)
            .expect("Cannot add more than MAX_MEMORY_REGIONS regions");
    }

    memory_map
}

/// Turns the framebuffer description back into a slice you can draw into.
///
/// # Safety
/// - `info.address` is non-null and a virtual address mapped in the current address space
///   (alignment is no concern: the element type is `u8`)
/// - The address range `info.address..info.address + info.byte_len` must be mapped as readable
///   and writable
/// - No aliasing allowed. While this slice exists nothing else may read or write that memory
///   through another reference or pointer
/// - The memory pointed by `info.address` must be valid for the whole duration of the kernel
/// - `info.byte_len` must not exceed `isize::MAX`
pub unsafe fn framebuffer_buffer(info: &FramebufferInfo) -> &'static mut [u8] {
    let address = info.address as *mut u8;
    let byte_len = info.byte_len;
    unsafe { core::slice::from_raw_parts_mut(address, byte_len) }
}
