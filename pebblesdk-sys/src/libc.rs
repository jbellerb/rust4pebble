use core::arch::asm;
use core::ffi::{c_uint, c_void};

unsafe extern "C" {
    pub fn malloc(size: c_uint) -> *mut c_void;
    pub fn calloc(count: c_uint, size: c_uint) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: c_uint) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    // pub fn memcmp(ptr1: *const c_void, ptr2: *const c_void, n: c_uint) -> c_int;
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: c_uint) -> *mut c_void;
    // pub fn memmove(dest: *mut c_void, src: *const c_void, n: c_uint) -> *mut c_void;
    // pub fn memset(dest: *mut c_void, c: c_int, n: c_uint) -> *mut c_void;
}

// These aliases are to override the compiler-provided implementations of memcpy
// and friends. Not that Rust's included implementations are bad, but PebbleOS
// provides them so it's an unneccesary increase in binary size. Since memcpy
// is treated specially by LLVM, an implementation _has_ to be given or else it
// will substitute with one before libpebble can even be linked. For the same
// reason, the declarations above are actually calling the below functions.
// Maybe I should just copy the trampoline code from the library instead?

#[no_mangle]
#[inline(never)]
pub unsafe extern "aapcs" fn __aeabi_memcpy(_dest: *mut u8, _src: *const u8, _n: usize) {
    // Since LLVM makes the above definition always refer to this, I have to
    // write manual assembly to prevent it from generating an infinite loop.
    // Ideally this would be a naked function, but I don't want to require a
    // nightly compiler to build this. Hopefully this continues to produce the
    // correct assembly.
    unsafe { asm!("b {}", sym memcpy, options(nomem, nostack, noreturn)) }
}

// TODO: memmove and memset (more complex since they both use different
// orderings for their arguments)
