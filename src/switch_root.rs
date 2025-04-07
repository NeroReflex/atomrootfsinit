#[cfg(target_arch = "arm")]
const SYS_PIVOT_ROOT: libc::c_long = 218;

#[cfg(target_arch = "aarch64")]
const SYS_PIVOT_ROOT: libc::c_long = 41;

#[cfg(target_arch = "x86_64")]
const SYS_PIVOT_ROOT: libc::c_long = 155;

pub fn pivot_root(new_root: &str, put_old: &str) -> Result<(), libc::c_int> {
    let new_root_str = crate::CStr::new(new_root)?;
    let put_old_str = crate::CStr::new(put_old)?;

    unsafe {
        /*
         * On success, zero is returned. On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::syscall(SYS_PIVOT_ROOT, new_root_str.inner(), put_old_str.inner()) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}

pub fn execute(program: &str) -> Result<(), libc::c_int> {
    let program_str = crate::CStr::new(program)?;

    let argv: [*const libc::c_char; 2] = [program_str.inner(), core::ptr::null()];

    unsafe {
        /*
         * On success, zero is returned. On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::execve(program_str.inner(), argv.as_ptr(), core::ptr::null()) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
