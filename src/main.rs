#![no_main]

extern crate libc;

use atomrootfsinit::{
    config::Config,
    mount::{MountFlag, Mountpoint, MountpointFlags},
    string::CStr,
    switch_root::switch_root,
};

pub(crate) struct CmdLine {
    root: Option<CStr>,
    init: Option<CStr>,
}

fn read_cmdline() -> Option<CmdLine> {
    match atomrootfsinit::read_whole_file("/proc/cmdline", atomrootfsinit::RDTAB_MAX_FILE_SIZE) {
        Ok(cmdline) => match core::str::from_utf8(cmdline.as_slice().unwrap()) {
            Ok(cmdline_str) => {
                let mut root = None;
                let mut init = None;
                for param in cmdline_str.split_ascii_whitespace() {
                    if param.starts_with("root=") {
                        root = Some(CStr::new(&param[5..param.len()]).unwrap_or_else(
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
                    } else if param.starts_with("init=") {
                        init = Some(CStr::new(&param[5..param.len()]).unwrap_or_else(
                            |err| unsafe {
                                libc::printf(
                                    b"Failed to store init software path: %d\n\0".as_ptr()
                                        as *const libc::c_char,
                                    err as libc::c_int,
                                );
                                libc::sleep(10);
                                libc::exit(err);
                            },
                        ));
                    }
                }

                Some(CmdLine { root, init })
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

    unsafe {
        libc::printf(b"\natomrootfsinit started\n\0".as_ptr() as *const libc::c_char);
    }

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
    .mount(&None)
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
    .mount(&None)
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

    // mount proc into /proc as rw so that /proc/cmdline and /proc/mounts will be accessible
    Mountpoint::new(
        Some("proc"),
        "/proc",
        Some("proc"),
        MountpointFlags::new(&[]),
        None,
    )
    .unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to create the mount object for /proc: %d\n\0".as_ptr()
                    as *const libc::c_char,
                err as libc::c_int,
            );
        }
        exit_error(err);

        unreachable!()
    })
    .mount(&None)
    .unwrap_or_else(|err| unsafe {
        libc::printf(
            b"Failed to mount /proc as private: %d\n\0".as_ptr() as *const libc::c_char,
            err as libc::c_int,
        );
    });

    let initramfs =
        atomrootfsinit::read_whole_file("/proc/mounts", atomrootfsinit::RDEXEC_MAX_FILE_SIZE)
            .unwrap_or_else(|err| {
                unsafe {
                    libc::printf(
                        b"Failed to open /proc/mounts: %d\n\0".as_ptr() as *const libc::c_char,
                        err as libc::c_int,
                    );
                }

                exit_error(err);

                unreachable!()
            })
            .split(b'\n', false)
            .unwrap_or_else(|err| {
                unsafe {
                    libc::printf(
                        b"Failed to split /proc/mounts by line: %d\n\0".as_ptr()
                            as *const libc::c_char,
                        err as libc::c_int,
                    );
                }

                exit_error(err);

                unreachable!()
            })
            .iter()
            .find_map(|raw_line| {
                let unsplitted_line = core::str::from_utf8(raw_line.as_slice().unwrap()).unwrap();

                let mut dev = "";
                let mut mount = "";
                for (idx, mount_component) in unsplitted_line.split(" ").enumerate() {
                    match idx {
                        0 => dev = mount_component,
                        1 => mount = mount_component,
                        _ => {}
                    }
                }

                if mount == "/" {
                    return Some(dev);
                }

                None
            })
            .map_or(false, |device| device == "rootfs");

    let cmdline = read_cmdline();

    let init = (match atomrootfsinit::read_whole_file(
        atomrootfsinit::RDEXEC_PATH,
        atomrootfsinit::RDEXEC_MAX_FILE_SIZE,
    ) {
        Ok(rdinit_content) => CStr::new(
            core::str::from_utf8(rdinit_content.as_slice().unwrap_or(&[]))
                .unwrap_or(atomrootfsinit::DEFAULT_INIT)
                .trim(),
        ),
        Err(err) => {
            unsafe {
                libc::printf(
                    b"Failed to open the rdinit file: %d -- default will be used\n\0".as_ptr()
                        as *const libc::c_char,
                    err as libc::c_int,
                );
            }

            match cmdline.as_ref().map_or(None, |a| a.init.clone()) {
                Some(init) => CStr::new(init.as_str()),
                None => CStr::new(atomrootfsinit::DEFAULT_INIT),
            }
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

    let mut rootfs_target = atomrootfsinit::SYSROOT;
    for mount in config.iter_mounts() {
        let rootfs = match mount.src() {
            Some(src) => match src {
                "rootdev" => {
                    rootfs_target = mount.target();
                    cmdline.as_ref().map_or(None, |a| a.root.clone())
                }
                _ => None,
            },
            None => None,
        };

        #[cfg(feature = "trace")]
        unsafe {
            libc::printf(
                b"Mounting %s\n\0".as_ptr() as *const libc::c_char,
                mount.target()
            );
        }

        if let Err(err) = mount.mount(&rootfs) {
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

    let rootfs_target = CStr::new(rootfs_target).unwrap_or_else(|err| {
        unsafe {
            libc::printf(
                b"Failed to allocate rootfs_target: %d\n\0".as_ptr() as *const libc::c_char,
                err as libc::c_int,
            );
        }

        exit_error(err);

        unreachable!()
    });

    // ensure memory is released before switch_root
    drop(config);

    if let Err(err) = switch_root(initramfs, rootfs_target.as_str(), ".", init.as_str()) {
        unsafe {
            libc::printf(
                b"Failed to switch_root to %s: %d\n\0".as_ptr() as *const libc::c_char,
                rootfs_target.inner(),
                err as libc::c_int,
            );
        }

        exit_error(1)
    }

    // This point is impossible to reach as switch_root calls execve
    // that replaces the current program with the specified one.
}

fn exit_error(err: libc::c_int) {
    #[cfg(feature = "droptosh")]
    if let Err(err) = atomrootfsinit::switch_root::execute("/bin/sh") {
        unsafe {
            libc::printf(
                b"Failed to execve the recovery/debug software: %d\n\0".as_ptr()
                    as *const libc::c_char,
                err as libc::c_int,
            );
        };
    }

    unsafe {
        libc::sleep(10);
        libc::exit(err)
    }
}
