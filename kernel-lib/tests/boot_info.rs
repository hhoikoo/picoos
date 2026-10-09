use kernel_lib::boot_info::{KernelStack, PAGE_SIZE};

#[test]
fn guard_page_is_the_page_directly_below_the_stack() {
    let stack = KernelStack {
        bottom: 0xffff_8000_0010_0000,
        len: 80 * 1024,
    };
    assert_eq!(
        stack.guard_page(),
        Some(0xffff_8000_000f_f000..0xffff_8000_0010_0000)
    );
}

#[test]
fn guard_page_has_the_size_of_one_page() {
    let guard = KernelStack {
        bottom: 0x20_0000,
        len: 4096,
    }
    .guard_page()
    .unwrap();
    assert_eq!(guard.end - guard.start, PAGE_SIZE);
}

#[test]
fn guard_page_exists_when_it_exactly_fits() {
    assert_eq!(
        KernelStack {
            bottom: PAGE_SIZE,
            len: 4096
        }
        .guard_page(),
        Some(0..PAGE_SIZE)
    );
}

#[test]
fn no_guard_page_when_there_is_no_room_below() {
    assert_eq!(
        KernelStack {
            bottom: PAGE_SIZE - 1,
            len: 4096
        }
        .guard_page(),
        None
    );
    assert_eq!(
        KernelStack {
            bottom: 0,
            len: 4096
        }
        .guard_page(),
        None
    );
}
