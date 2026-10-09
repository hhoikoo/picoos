//! The PicoOS kernel library. The binary (`main.rs`) and the QEMU tests (`tests/`) share this crate.

#![no_std]

pub mod boot;

use bootloader_api::BootloaderConfig;
use bootloader_api::config::Mapping;

/// Boot-time configuration for the `bootloader` crate: map all physical memory at a dynamic
/// offset and keep dynamic mappings (stack, boot info, framebuffer, physical memory) in the
/// higher half so user space can later own the lower half.
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.mappings.dynamic_range_start = Some(0xffff_8000_0000_0000);
    config
};
