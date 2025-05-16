use crate::string::CStr;

pub fn create_hardlink(src: &str, dst: &str) -> Result<(), libc::c_int> {
    let src = CStr::new(src)?;
    let dst = CStr::new(dst)?;

    unsafe {
        /*
         * On success, zero is returned. On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::link(src.inner(), dst.inner()) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
