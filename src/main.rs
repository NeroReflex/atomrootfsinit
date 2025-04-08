#![no_main]

extern crate libc;

use atombutter::{
    change_dir::chdir, config::Config, mount::{direct_detach, MountFlag, Mountpoint, MountpointFlags}, switch_root::{execute, pivot_root}
};

#[no_mangle]
#[inline(never)]
fn main() {
    const SLASH: &str = "/";

    /* Work-around for kernel design: the kernel refuses MS_MOVE if any file systems are mounted
     * MS_SHARED. Hence remount them MS_PRIVATE here as a work-around.
     *
     * https://bugzilla.redhat.com/show_bug.cgi?id=847418
     */
    let priv_root_mountpoint = Mountpoint::new(
        None,
        SLASH.as_bytes(),
        None,
        MountpointFlags::new(&[MountFlag::Recursive, MountFlag::Private]),
        None,
    )
    .unwrap_or_else(|err| unsafe {
        libc::printf(
            b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
            err as libc::c_int,
        );
        libc::sleep(600);
        libc::exit(1);
    });

    let config = Config::new("/etc/bstab").unwrap_or_else(|err| unsafe {
        libc::printf(
            b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
            err as libc::c_int,
        );
        libc::sleep(600);
        libc::exit(1);
    });

    if let Err(err) = priv_root_mountpoint.mount() {
        unsafe {
            libc::printf(
                b"Failed to remount / as private: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = chdir(atombutter::SYSROOT) {
        unsafe {
            libc::printf(
                b"Failed to chdir /sysroot: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = pivot_root(".", ".") {
        unsafe {
            libc::printf(
                b"Failed to pivot root to /sysroot: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = direct_detach(".") {
        unsafe {
            libc::printf(
                b"Failed to umount the old rootfs: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = chdir(SLASH) {
        unsafe {
            libc::printf(
                b"Failed to chdir to the new rootfs: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = execute(atombutter::INIT) {
        unsafe {
            libc::printf(
                b"Failed to execve the init program: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else {
        // This point should never be reached as execute calls execve
        // that replaces the current program with the specified one.
        unreachable!();
    }

    // If we ends up here let the user know about that as this shouldn't happen
    unsafe {
        libc::sleep(800);
        libc::exit(1);
    }
}
