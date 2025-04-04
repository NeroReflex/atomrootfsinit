pub fn chdir(path: &str) -> Result<(), libc::c_int> {
    unsafe {
        /*
         * On success, zero is returned.  On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::chdir(path.as_ptr() as *const libc::c_char) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
