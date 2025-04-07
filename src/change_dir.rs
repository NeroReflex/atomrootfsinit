pub fn chdir(path: &str) -> Result<(), libc::c_int> {
    let path_str = crate::CStr::new(path)?;

    unsafe {
        /*
         * On success, zero is returned.  On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::chdir(path_str.inner()) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
