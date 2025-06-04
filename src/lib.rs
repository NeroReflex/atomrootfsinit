#![no_std]
#![no_main]

extern crate libc;

pub mod change_dir;
pub mod config;
pub mod link;
pub mod mount;
pub mod string;
pub mod switch_root;
pub mod vector;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SYSROOT: &str = "/mnt";

pub const DEFAULT_INIT: &str = "/sbin/init";

pub const RDNAME_PATH: &str = "/etc/rdname";
pub const RDNAME_MAX_FILE_SIZE: usize = 256;

pub const RDEXEC_PATH: &str = "/etc/rdexec";
pub const RDEXEC_MAX_FILE_SIZE: usize = 256;

pub const RDTAB_PATH: &str = "/mnt/etc/rdtab";
pub const RDTAB_MAX_FILE_SIZE: usize = 16384;

pub fn check_file_exists(path: &str) -> Result<bool, libc::c_int> {
    let path_str = crate::string::CStr::new(path)?;

    Ok(unsafe { libc::access(path_str.inner(), 0) } == libc::F_OK)
}

pub fn read_whole_file(
    path: &str,
    max_file_size: usize,
) -> Result<crate::vector::Vec<u8>, libc::c_int> {
    let mut content = crate::vector::Vec::<u8>::with_capacity(max_file_size)?;

    let path_str = crate::string::CStr::new(path)?;

    let content = unsafe {
        let fd = libc::open(path_str.inner(), libc::O_RDONLY);
        if fd < 0 {
            return Err(*libc::__errno_location());
        }

        if let Err(err) = content.fill_by_function(|ptr, capacity| {
            let bytes_read = libc::read(fd, ptr as *mut libc::c_void, capacity);

            if bytes_read < 0 {
                return Err(*libc::__errno_location());
            }

            Ok(bytes_read as usize)
        }) {
            libc::close(fd);
            return Err(err);
        }

        libc::close(fd);
        content
    };

    Ok(content)
}
