use crate::string::CStr;

#[cfg(target_arch = "arm")]
const SYS_PIVOT_ROOT: libc::c_long = 218;

#[cfg(target_arch = "aarch64")]
const SYS_PIVOT_ROOT: libc::c_long = 41;

#[cfg(target_arch = "x86_64")]
const SYS_PIVOT_ROOT: libc::c_long = 155;

pub fn pivot_root(new_root: &str, put_old: &str) -> Result<(), libc::c_int> {
    let new_root_str = CStr::new(new_root)?;
    let put_old_str = CStr::new(put_old)?;

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
    let program_str = CStr::new(program)?;

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

/**
 * Function to perform either pivot_root or switch_root depending on the new_root.
 * 
 * initrd is a real filesystem (like ext2) and it can be unmounted,
 * therefore sys_pivot_root syscall is to be used.
 * 
 * initramfs is not a real filesystem therefore the new root can be /
 * and sys_pivot_root cannot be used: remount the root device as / instead.
 * 
 * @param new_root where the root device is mounted: if "/" is used sys_pivot_root will NOT
 * be performed
 * @param put_old the second parameter for sys_pivot_root
 * @param program the init program to be execve'd on the new root
 */
pub fn switch_root(new_root: &str, put_old: &str, program: &str) -> Result<(), libc::c_int> {
    todo!()
}