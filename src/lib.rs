#![no_std]

extern crate libc;

pub mod change_dir;
pub mod config;
pub mod mount;
pub mod switch_root;

pub const SYSROOT: &str = "/sysroot";
pub const PUT_OLD: &str = "/sysroot/mnt";

pub const INIT: &str = "/usr/lib/systemd/systemd";

pub struct CStr {
    alloc_sz: usize,
    data: *const libc::c_char,
}

impl Drop for CStr {
    fn drop(&mut self) {
        unsafe {
            if self.data != core::ptr::null_mut() {
                libc::free(self.data as *mut libc::c_void)
            }
        }
    }
}

impl CStr {
    pub fn new(str: &str) -> Result<Self, libc::c_int> {
        let true_str_len = str.find('\0').unwrap_or(str.len());

        let alloc_sz = true_str_len + 1;
        let data = unsafe { libc::malloc(alloc_sz) } as *mut libc::c_char;

        if data == core::ptr::null_mut() {
            return Err(libc::ENOMEM);
        }

        unsafe {
            libc::memset(data as *mut libc::c_void, 0, alloc_sz);
            let _ = libc::memcpy(data as *mut libc::c_void, str.as_ptr() as *const libc::c_void, true_str_len);
        }

        Ok(Self { alloc_sz, data })
    }

    pub fn strlen(&self) -> usize {
        /*
        for i in 0..self.alloc_sz {
            if unsafe { *self.data.offset(i as isize) } == 0 {
                return i;
            }
        }
        */

        self.alloc_sz - 1
    }

    pub fn inner(&self) -> *const libc::c_char {
        self.data
    }
}
