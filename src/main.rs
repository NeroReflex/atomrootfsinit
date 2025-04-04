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
    let priv_root_mountpoint = match Mountpoint::new(
        None,
        SLASH,
        None,
        MountpointFlags::new(&[MountFlag::Recursive, MountFlag::Private]),
    ) {
        Ok(mount) => mount,
        Err(err) => {
            eprintln!("Failed to create the mount object: {err}");
            // If we ends up here let the user know about that as this shouldn't happen
            loop {
                unsafe {
                    libc::sleep(1);
                }
            }
        }
    };

    match priv_root_mountpoint.mount() {
        Ok(_) => match pivot_root(atombutter::SYSROOT, atombutter::PUT_OLD) {
            Ok(_) => match chdir(SLASH) {
                Ok(_) => match execute(atombutter::INIT) {
                    Ok(_) => {
                        /* execute calls execve that replaces this program with the specified one */
                        unreachable!()
                    }
                    Err(err) => eprintln!("Failed to execve the init program: {err}"),
                },
                Err(err) => eprintln!("Failed to chdir to the new rootfs: {err}"),
            },
            Err(err) => eprintln!("Failed to pivot root to /sysroot: {err}"),
        },
        Err(err) => eprintln!("Failed to remount / as private: {err}"),
    }

    // If we ends up here let the user know about that as this shouldn't happen
    loop {
        unsafe {
            libc::sleep(1);
        }
    }
}
