//! Hardware-independent logic for the kernel, kept in a plain crate so `cargo test` can run it on the host.

#![no_std]

pub mod boot_info;
pub mod framebuffer;
