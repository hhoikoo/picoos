//! The kernel's own description of what the bootloader handed over.
//!
//! Every other part of the kernel uses these types and never `bootloader_api`'s, so a different
//! bootloader (our own, in Phase 2) only has to replace the one adapter function in
//! `kernel::boot`. The plain data types here are given; your tasks are marked `todo!()`.

use core::ops::Range;

/// Size of a page in bytes.
pub const PAGE_SIZE: u64 = 4096;

/// How many memory regions the kernel keeps. The adapter panics if the bootloader reports more.
pub const MAX_MEMORY_REGIONS: usize = 128;

/// How the bytes of one pixel are laid out. Any bytes of a pixel beyond the colour channels are padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Bytes are `[red, green, blue, ...]`.
    Rgb,
    /// Bytes are `[blue, green, red, ...]`.
    Bgr,
    /// One brightness byte, then padding.
    Gray,
    /// Anything else; drawing code must leave such a framebuffer alone.
    Unknown,
}

/// Layout of the framebuffer (the memory that is shown on screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferInfo {
    /// Virtual address of the first byte.
    pub address: u64,
    /// Total size in bytes.
    pub byte_len: usize,
    /// Visible width in pixels.
    pub width: usize,
    /// Visible height in pixels.
    pub height: usize,
    /// Pixels (not bytes!) from the start of one row to the start of the next; `>= width`.
    pub stride: usize,
    /// Bytes per pixel, padding included.
    pub bytes_per_pixel: usize,
    /// Channel layout of a pixel.
    pub pixel_format: PixelFormat,
}

/// What a range of physical memory may be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    /// Free for the kernel to use.
    Usable,
    /// In use by the bootloader or the firmware, or unknown. Never touch it.
    Reserved,
}

/// A range of physical memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Physical start address.
    pub start: u64,
    /// Length in bytes.
    pub len: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    const EMPTY: Self = Self {
        start: 0,
        len: 0,
        kind: MemoryRegionKind::Reserved,
    };
}

/// A fixed-capacity list of memory regions (there is no heap yet).
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    len: usize,
}

impl MemoryMap {
    pub const fn new() -> Self {
        Self {
            regions: [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS],
            len: 0,
        }
    }

    /// Appends a region; gives it back if the map is full.
    pub fn push(&mut self, region: MemoryRegion) -> Result<(), MemoryRegion> {
        let slot = self.regions.get_mut(self.len).ok_or(region)?;
        *slot = region;
        self.len += 1;
        Ok(())
    }

    pub fn as_slice(&self) -> &[MemoryRegion] {
        &self.regions[..self.len]
    }
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Where the kernel image lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelImage {
    /// Physical address of the ELF file as loaded.
    pub phys_addr: u64,
    /// Virtual address the image is mapped at.
    pub virt_base: u64,
    /// Size in bytes.
    pub len: u64,
}

/// The stack the kernel started on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelStack {
    /// Lowest *mapped* virtual address of the stack.
    pub bottom: u64,
    /// Size in bytes. The stack grows down from `bottom + len` towards `bottom`.
    pub len: u64,
}

impl KernelStack {
    /// The virtual address range of the guard page: the single unmapped page directly below
    /// `bottom`, so a stack overflow faults instead of silently corrupting memory.
    pub fn guard_page(&self) -> Option<Range<u64>> {
        self.bottom
            .checked_sub(PAGE_SIZE)
            .map(|top| top..self.bottom)
    }
}

/// Everything the kernel needs to know about the machine it was booted on.
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// `None` if the machine has no framebuffer.
    pub framebuffer: Option<FramebufferInfo>,
    pub memory_map: MemoryMap,
    /// All physical memory is mapped at `physical_memory_offset + physical_address`.
    pub physical_memory_offset: u64,
    pub kernel_image: KernelImage,
    pub kernel_stack: KernelStack,
}
