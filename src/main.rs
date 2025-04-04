extern crate libc;

use atombutter::{
    change_dir::chdir,
    mount::{MountFlag, Mountpoint, MountpointFlags},
    switch_root::{execute, pivot_root},
};

fn main() {
    const SLASH: &str = "/\0";

    /* Work-around for kernel design: the kernel refuses MS_MOVE if any file systems are mounted
     * MS_SHARED. Hence remount them MS_PRIVATE here as a work-around.
     *
     * https://bugzilla.redhat.com/show_bug.cgi?id=847418
     */
    let priv_root_mountpoint = Mountpoint::new(
        None,
        SLASH,
        None,
        MountpointFlags::new(&[MountFlag::Recursive, MountFlag::Private]),
    ).unwrap_or_else(|err| {
        eprintln!("Failed to create the mount object: {err}");
        loop { unsafe { libc::sleep(1); } }
    });

    if let Err(err) = priv_root_mountpoint.mount() {
        eprintln!("Failed to remount / as private: {err}");
    } else if let Err(err) = pivot_root(atombutter::SYSROOT, atombutter::PUT_OLD) {
        eprintln!("Failed to pivot root to /sysroot: {err}");
    } else if let Err(err) = chdir(SLASH) {
        eprintln!("Failed to chdir to the new rootfs: {err}");
    } else if let Err(err) = execute(atombutter::INIT) {
        eprintln!("Failed to execve the init program: {err}");
    } else {
        // This point should never be reached as execute calls execve
        // that replaces the current program with the specified one.
        unreachable!();
    }

    // If we ends up here let the user know about that as this shouldn't happen
    loop { unsafe { libc::sleep(1); } }
}
