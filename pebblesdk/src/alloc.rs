use core::alloc::{GlobalAlloc, Layout};
use core::cmp::max;
use core::ffi::c_void;

use pebblesdk_sys::libc::{calloc, free, malloc, realloc};

struct PebbleLibcAlloc {}

unsafe impl Sync for PebbleLibcAlloc {}

// The PebbleOS malloc implementation (src/libutil/heap.c) guarantees alignment
// of sizeof(unsigned long). This function calculates the smallest possible
// 4-byte aligned chunk guaranteed to contain the requested layout with at least
// 4 bytes before it (to contain the allocation pointer needed for free).
//
// # Panics
//
// Panics if the resulting size would exceed `isize::MAX`.
fn pad_for_alignment(layout: Layout) -> Layout {
    let align = max(layout.align(), size_of::<*mut c_void>());
    let size = layout.size();

    // Ironically, the worst case is that malloc gives us something perfectly
    // aligned to the alignment, since we'll have no room for the  allocation
    // pointer unless we shift it over an entire other alignment. Therefore, we
    // need to request one alignment more than the smallest multiple of align
    // greater or equal to size.

    // This puts the upper bound on our allocation's size at `isize::MAX + 1`
    // (`Layout` guarantees the size rounded up to the nearest multiple of align
    // doesn't overflow, so `align + size < isize::MAX + 1`), which does not
    // overflow `usize`, but does break the invariant of `Layout`.
    assert!(
        align + size <= isize::MAX as usize,
        "Padded alignment would overflow!"
    );

    // Since align is guaranteed to be a power of two, we can find this with bit
    // masking. Removing the bits of `align - 1` gives a multiple of align. To
    // make sure it is the _second_ smallest multiple greater or equal to size,
    // add `2 * align - 1` before masking. To avoid overflow, we can add the
    // second align after masking.
    let mask = align - 1;
    let padded_size = ((layout.size() + mask) & !mask) + align;

    unsafe { Layout::from_size_align_unchecked(padded_size, align) }
}

// Find the lowest aligned address in the allocation with room before it to
// store a pointer, write the original pointer before it, return an aligned
// pointer.
unsafe fn mark_allocation(ptr: *mut c_void, align: usize) -> *mut u8 {
    let offset = ptr.align_offset(align);
    unsafe {
        let start = ptr.add(offset) as *mut *mut c_void;
        *(start.sub(1)) = ptr;
        start as *mut u8
    }
}

unsafe impl GlobalAlloc for PebbleLibcAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let aligned = pad_for_alignment(layout);
        let ptr = unsafe { malloc(aligned.size() as u32) } as *mut c_void;
        if ptr.is_null() {
            return ptr as *mut u8;
        }
        mark_allocation(ptr, aligned.align())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let aligned = pad_for_alignment(layout);
        let ptr = unsafe { calloc(1, aligned.size() as u32) } as *mut c_void;
        if ptr.is_null() {
            return ptr as *mut u8;
        }
        mark_allocation(ptr, aligned.align())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // The caller is required by the trait to guarantee this does not
        // have a size greater than `isize::MAX` when rounded up to the
        // nearest align.
        let aligned = unsafe {
            pad_for_alignment(Layout::from_size_align_unchecked(new_size, layout.align()))
        };
        let ptr = unsafe {
            let old_ptr = *((ptr as *mut *mut c_void).sub(1));
            realloc(old_ptr as *mut c_void, aligned.size() as u32) as *mut c_void
        };
        if ptr.is_null() {
            return ptr as *mut u8;
        }
        mark_allocation(ptr, aligned.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            let padded_ptr = *((ptr as *mut *mut c_void).sub(1));
            free(padded_ptr as *mut c_void);
        }
    }
}

/// A global allocator backed by the heap provided by PebbleOS.
#[global_allocator]
static ALLOCATOR: PebbleLibcAlloc = PebbleLibcAlloc {};
