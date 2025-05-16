#![no_main]

extern crate libc;

use atomrootfsinit::{
    change_dir::chdir,
    config::Config,
    link::create_hardlink,
    mount::{direct_detach, MountFlag, Mountpoint, MountpointFlags},
    string::CStr,
    switch_root::{execute, pivot_root},
};

fn find_dev_from_mountpoint(mount_point: &str) -> Result<Option<CStr>, libc::c_int> {
    // Open /proc/mounts to find the device associated with /mnt
    let mountpoints = atomrootfsinit::read_whole_file("/proc/mounts", 16384)?;
    let raw_data = mountpoints.split(b'\n', false).unwrap();
    for mount_entry_line in raw_data.iter() {
        match core::str::from_utf8(mount_entry_line.as_slice().unwrap()) {
            Ok(line) => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == mount_point {
                    return Ok(Some(CStr::new(parts[0])?));
                }
            }
            Err(_err) => unsafe {
                libc::printf(
                    b"Failed converting mount entry line to utf-8: ignoring the line\n\0".as_ptr()
                        as *const libc::c_char,
                );
            },
        }
    }

    Ok(None)
}

fn read_rootdev() -> Option<CStr> {
    match atomrootfsinit::read_whole_file("/proc/cmdline", atomrootfsinit::RDTAB_MAX_FILE_SIZE) {
        Ok(cmdline) => match core::str::from_utf8(cmdline.as_slice().unwrap()) {
            Ok(cmdline_str) => {
                let mut rootdev = None;
                for param in cmdline_str.split_ascii_whitespace() {
                    if param.starts_with("root=") {
                        rootdev = Some(CStr::new(&param[5..param.len()]).unwrap_or_else(
                            |err| unsafe {
                                libc::printf(
                                    b"Failed to store root device name: %d\n\0".as_ptr()
                                        as *const libc::c_char,
                                    err as libc::c_int,
                                );
                                libc::sleep(10);
                                libc::exit(err);
                            },
                        ));

                        break;
                    }
                }

                rootdev
            }
            Err(_err) => unsafe {
                libc::printf(
                    b"Failed to convert cmdline to utf-8\n\0".as_ptr() as *const libc::c_char
                );
                libc::sleep(10);

                None
            },
        },
        Err(err) => unsafe {
            libc::printf(
                b"Failed to read kernel cmdline: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );

            None
        },
    }
}

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
    Mountpoint::new(
        None,
        SLASH,
        None,
        MountpointFlags::new(&[MountFlag::Recursive, MountFlag::Private]),
        None,
    )
    .unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
        exit_error(err);

        unreachable!()
    })
    .mount(None)
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
        libc::sigprocmask(
            libc::SIG_BLOCK,
            &set,
            std::ptr::null_mut::<libc::sigset_t>(),
        );
    }

    (match atomrootfsinit::read_whole_file(
        atomrootfsinit::RDNAME_PATH,
        atomrootfsinit::RDNAME_MAX_FILE_SIZE,
    ) {
        Ok(mut rdname_content) => {
            rdname_content.push(0u8).unwrap_or_else(|err| {
                unsafe {
                    libc::printf(
                        b"Failed to append NUL-terminator to rdname content %s: %d\n\0".as_ptr()
                            as *const libc::c_char,
                        rdname_content.as_slice(),
                        err as libc::c_int,
                    );
                }
                exit_error(err);

                unreachable!()
            });

            rdname_content
                .prepend(b"/deployments/")
                .unwrap_or_else(|err| {
                    unsafe {
                        libc::printf(
                            b"Failed to get the temporary path to the deployment %s: %d\n\0"
                                .as_ptr() as *const libc::c_char,
                            rdname_content.as_slice(),
                            err as libc::c_int,
                        );
                    }
                    exit_error(err);

                    unreachable!()
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
                    if (val == b'\t') || (val == b'\n') || (val == b' ') {
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
                            .unwrap_or(""),
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
    .unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to create the mount object: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
        exit_error(err);

        unreachable!()
    })
    .mount(None)
    .unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to mount /mnt: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }
        exit_error(err);

        unreachable!()
    });

    let init = (match atomrootfsinit::read_whole_file(
        atomrootfsinit::RDEXEC_PATH,
        atomrootfsinit::RDEXEC_MAX_FILE_SIZE,
    ) {
        Ok(rdinit_content) => CStr::new(
            core::str::from_utf8(rdinit_content.as_slice().unwrap_or(&[]))
                .unwrap_or(atomrootfsinit::SYSTEMD_INIT),
        ),
        Err(err) => {
            #[cfg(debug_assertions)]
            unsafe {
                libc::printf(
                    b"Failed to open the rdinit file: %d -- systemd will be used\n\0".as_ptr()
                        as *const libc::c_char,
                    err as libc::c_int,
                );
            }

            CStr::new(atomrootfsinit::SYSTEMD_INIT)
        }
    })
    .unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to allocate init: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }

        exit_error(err);

        unreachable!()
    });

    let config = match atomrootfsinit::read_whole_file(
        atomrootfsinit::RDTAB_PATH,
        atomrootfsinit::RDTAB_MAX_FILE_SIZE,
    ) {
        Ok(rdinit_content) => Config::new(rdinit_content).unwrap_or_else(|err| {
            unsafe {
                libc::printf(
                    b"Failed to parse configuration: %d\n\0".as_ptr() as *const libc::c_char,
                    err as libc::c_int,
                );
            }
            exit_error(err);
            unreachable!()
        }),
        Err(err) => {
            unsafe {
                libc::printf(
                    b"Failed to read configuration file: %d\n\0".as_ptr() as *const libc::c_char,
                    err as libc::c_int,
                );
            }
            return exit_error(err);
        }
    };

    // TODO: mount proc into /proc as rw so that /proc/cmdline and /proc/mountpoints will be accessible

    for mount in config.iter_mounts() {
        let rootfs = match mount.src() {
            Some(src) => match src {
                "rootdev" => read_rootdev(),
                _ => None,
            },
            None => None,
        };

        if let Err(err) = mount.mount(rootfs) {
            unsafe {
                libc::printf(
                    b"Failed to mount %s: %d\n\0".as_ptr() as *const libc::c_char,
                    mount.target(),
                    err as libc::c_int,
                );
            }

            return exit_error(err);
        }
    }

    // ensure memory is released before switch_root
    drop(config);

    // link /dev/root to the root device if one is specified in kernel cmdline and proc is mounted
    let rootdev = match read_rootdev() {
        Some(rootdev) => rootdev,
        None => match find_dev_from_mountpoint(atomrootfsinit::SYSROOT) {
            Ok(rootdev) => match rootdev {
                Some(rootdev) => rootdev,
                None => {
                    unsafe {
                        libc::printf(
                            b"No root device specified\n\0".as_ptr() as *const libc::c_char
                        );
                    }

                    return exit_error(libc::ENODEV);
                }
            },
            Err(err) => {
                unsafe {
                    libc::printf(
                        b"Failed to get root device: %d\n\0".as_ptr() as *const libc::c_char,
                        err as libc::c_int,
                    );
                };

                return exit_error(err);
            }
        },
    };

    if let Err(err) = create_hardlink(rootdev.as_str(), "/dev/root") {
        unsafe {
            libc::printf(
                b"Failed to create /dev/root link to root device: %d\n\0".as_ptr()
                    as *const libc::c_char,
                err as libc::c_int,
            );
        }
    } else if let Err(err) = chdir(atomrootfsinit::SYSROOT) {
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
    } else if let Err(err) = execute(init.as_str()) {
        unsafe {
            libc::printf(
                b"Failed to execve the init program %s: %d\n\0".as_ptr() as *const libc::c_char,
                init.inner(),
                err as libc::c_int,
            );
        }
    } else {
        // This point should never be reached as execute calls execve
        // that replaces the current program with the specified one.
        unreachable!();
    }

    exit_error(1)
}

fn exit_error(err: libc::c_int) {
    if let Err(err) = execute("/bin/bash") {
        unsafe {
            libc::printf(
                b"Failed to execve the recovery/debug software: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        };
    }

    unsafe {
        libc::sleep(10);
        libc::exit(err)
    }
}
