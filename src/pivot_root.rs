#[cfg(target_arch = "arm")]
const SYS_PIVOT_ROOT: libc::c_long = 218;

#[cfg(target_arch = "aarch64")]
const SYS_PIVOT_ROOT: libc::c_long = 41;

#[cfg(target_arch = "x86_64")]
const SYS_PIVOT_ROOT: libc::c_long = 155;

pub fn switch_root(new_root: &str, put_old: &str) -> Result<(), libc::c_int> {
    unsafe {
        /*
         * On success, zero is returned.  On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::syscall(
            SYS_PIVOT_ROOT,
            new_root.as_ptr() as *const libc::c_char,
            put_old.as_ptr() as *const libc::c_char,
        ) != 0
        {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}

pub fn execute(program: &str) -> Result<(), libc::c_int> {
    let argv: [*const libc::c_char; 2] =
        [program.as_ptr() as *const libc::c_char, core::ptr::null()];

    unsafe {
        /*
         * On success, zero is returned.  On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::execve(
            program.as_ptr() as *const libc::c_char,
            argv.as_ptr(),
            core::ptr::null(),
        ) != 0
        {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
