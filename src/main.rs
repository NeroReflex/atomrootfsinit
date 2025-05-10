#![no_main]

extern crate libc;

use atomrootfsmgr::{
    change_dir::chdir,
    config::Config,
    mount::{direct_detach, MountFlag, Mountpoint, MountpointFlags},
    switch_root::{execute, pivot_root},
};

#[no_mangle]
#[inline(never)]
fn main() {
    const SLASH: &str = "/";

    /*
     * Work-around for kernel design: the kernel refuses MS_MOVE if any file systems are mounted
     * MS_SHARED. Hence remount them MS_PRIVATE here as a work-around.
     *
     * https://bugzilla.redhat.com/show_bug.cgi?id=847418
     */
    #[cfg(target_os = "linux")]
        let _ = Mountpoint::new(
            None,
            SLASH,
            None,
            MountpointFlags::new(&[MountFlag::Recursive, MountFlag::Private]),
            None,
        )
        .unwrap_or_else(|err| unsafe {
            libc::printf(
                b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
            libc::sleep(10);
            libc::exit(err);
        })
        .mount()
        .unwrap_or_else(|err| unsafe {
            libc::printf(
                b"Failed to remount / as private: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        });

    unsafe {
        // Create a signal set and fill it
        let mut set: libc::sigset_t = core::mem::zeroed();
        libc::sigfillset(&mut set);

        // Block all signals
        libc::sigprocmask(libc::SIG_BLOCK, &set, 0 as *mut libc::sigset_t);
    }

    (match atomrootfsmgr::read_whole_file(atomrootfsmgr::RDNAME_PATH, atomrootfsmgr::RDNAME_MAX_FILE_SIZE) {
        Ok(mut rdname_content) => {
            rdname_content.push(0u8).unwrap_or_else(|err| unsafe {
                libc::printf(
                    b"Failed to append NUL-terminator to rdname content %s: %d\n\0".as_ptr()
                        as *const libc::c_char,
                    rdname_content.as_slice(),
                    err as libc::c_int,
                );
                libc::sleep(10);
                libc::exit(err);
            });

            rdname_content
                .prepend(b"/deployments/")
                .unwrap_or_else(|err| unsafe {
                    libc::printf(
                        b"Failed to get the temporary path to the deployment %s: %d\n\0".as_ptr()
                            as *const libc::c_char,
                        rdname_content.as_slice(),
                        err as libc::c_int,
                    );
                    libc::sleep(10);
                    libc::exit(err);
                });

            'trim: loop {
                let curr_len = rdname_content.len();
                if curr_len == 0 {
                    unsafe {
                        libc::printf(b"File rdname does not contain a valid name!\n\0".as_ptr()
                            as *const libc::c_char)
                    };
                    break 'trim;
                }

                if let Some(val) = rdname_content.at(curr_len - 1) {
                    if (val == ('\t' as u8)) || (val == ('\n' as u8)) || (val == (' ' as u8)) {
                        match rdname_content.pop() {
                            Some(ch) => unsafe {
                                libc::printf(
                                    b"debug: pop %02x\n\0".as_ptr() as *const libc::c_char,
                                    ch as libc::c_uint,
                                );
                            },
                            None => unreachable!(),
                        }
                        continue 'trim;
                    }
                }

                break 'trim;
            }

            match rdname_content.empty() {
                true => Mountpoint::new(
                    Some(SLASH),
                    "/mnt",
                    Some("bind"),
                    MountpointFlags::new(&[MountFlag::Bind]),
                    None,
                ),
                false => Mountpoint::new(
                    Some(
                        core::str::from_utf8(rdname_content.as_slice().unwrap_or(&[]))
                            .unwrap_or_else(|_| ""),
                    ),
                    "/mnt",
                    Some("bind"),
                    MountpointFlags::new(&[MountFlag::Bind]),
                    None,
                ),
            }
        }
        Err(err) => {
            unsafe {
                libc::printf(
                    b"Couldn't read rdname file: %d -- / will be the rootfs\n\0".as_ptr()
                        as *const libc::c_char,
                    err as libc::c_int,
                )
            };

            Mountpoint::new(
                Some(SLASH),
                "/mnt",
                Some("bind"),
                MountpointFlags::new(&[MountFlag::Bind]),
                None,
            )
        }
    })
    .unwrap_or_else(|err| unsafe {
        libc::printf(
            b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
            err as libc::c_int,
        );
        libc::sleep(10);
        libc::exit(err);
    })
    .mount()
    .unwrap_or_else(|err| unsafe {
        libc::printf(
            b"Failed to mount /mnt: %d\n\0".as_ptr() as *const libc::c_char,
            err as libc::c_int,
        );
        libc::sleep(10);
        libc::exit(err);
    });

    unsafe { libc::printf(b"Reading configuration...\n\0".as_ptr() as *const libc::c_char) };

    let config = match atomrootfsmgr::read_whole_file(
        atomrootfsmgr::RDTAB_PATH,
        atomrootfsmgr::RDTAB_MAX_FILE_SIZE,
    ) {
        Ok(rdinit_content) => Config::new(rdinit_content).unwrap_or_else(|err| unsafe {
            libc::printf(
                b"Failed to parse configuration: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
            libc::sleep(10);
            libc::exit(err);
        }),
        Err(err) => unsafe {
            libc::printf(
                b"Failed to read configuration file: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
            libc::sleep(10);
            libc::exit(err);
        },
    };

    for mount in config.iter_mounts() {
        if let Err(err) = mount.mount() {
            unsafe {
                libc::printf(
                    b"Failed to mount %s: %d\n\0".as_ptr() as *const libc::c_char,
                    mount.target(),
                    err as libc::c_int,
                );
                libc::sleep(10);
                libc::exit(err);
            }
        }
    }

    if let Err(err) = chdir(atomrootfsmgr::SYSROOT) {
        unsafe {
            libc::printf(
                b"Failed to chdir /mnt: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = pivot_root(".", ".") {
        unsafe {
            libc::printf(
                b"Failed to pivot root to /mnt: %d\n\0".as_ptr() as *const libc::c_char,
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
    } else if let Err(err) = execute(atomrootfsmgr::INIT) {
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
        libc::printf(b"An unrecognised error has happened\n\0".as_ptr() as *const libc::c_char);
        libc::sleep(10);
        libc::exit(1)
    }
}
