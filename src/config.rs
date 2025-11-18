use crate::mount::{MountFlag, Mountpoint, MountpointFlags};
use crate::vector::Vec;

pub struct Config {
    mounts: Vec<Mountpoint>,
}

fn serialized_flags_split(
    serialized_flags: &str,
) -> Result<(MountpointFlags, Vec<u8>), libc::c_int> {
    let mut flags = MountpointFlags::default();
    let mut data = Vec::<u8>::default();

    for flag in serialized_flags.split(',') {
        match flag {
            "rw" => {}
            "nodev" => flags.set(MountFlag::NoDev),
            "noexec" => flags.set(MountFlag::NoExec),
            "nosuid" => flags.set(MountFlag::NoSUID),
            "noatime" => flags.set(MountFlag::NoAccessTime),
            "remount" => flags.set(MountFlag::Remount),
            "recursive" => flags.set(MountFlag::Recursive),
            "lazytime" => flags.set(MountFlag::Lazytime),
            "silent" => flags.set(MountFlag::Silent),
            "shared" => flags.set(MountFlag::Shared),
            "private" => flags.set(MountFlag::Private),
            "bind" => flags.set(MountFlag::Bind),
            "ro" => flags.set(MountFlag::ReadOnly),
            flg => {
                for d in flg.as_bytes().iter() {
                    data.push(*d)?;
                }
            }
        }

        data.push(b',')?;
    }

    // mount flags are given as C string to the kernel: ensure it is NULL-terminated
    if !data.empty() {
        // remove the last (unused) ','
        let _ = data.pop();

        // data can be a pointer to a kernel-defined struct,
        // but most filesystems in linux accepts a C-like string:
        // make sure such a string is NUL-terminated
        data.push(0u8)?;
    }

    Ok((flags, data))
}

impl Config {
    pub fn new(content: Vec<u8>) -> Result<Self, libc::c_int> {
        let mut mounts = Vec::<Mountpoint>::default();

        let raw_data = content.split(b'\n', false)?;
        drop(content);

        for mount_entry_line in raw_data.iter() {
            for (idx, unsplitted_line) in core::str::from_utf8(mount_entry_line.as_slice().unwrap())
                .map_err(|_| libc::EINVAL)?
                .split("#")
                .enumerate()
            {
                if idx > 0 || unsplitted_line.is_empty() {
                    break;
                }

                let mut index = 0;
                let mut src: Option<&str> = None;
                let mut target: Option<&str> = None;
                let mut fstype: Option<&str> = None;
                let mut flags = MountpointFlags::default();
                let mut data = Vec::default();
                let mut _dump: libc::c_uint = 0;
                let mut _fsck: libc::c_uint = 0;
                for mount_entry_param in unsplitted_line.split(" ") {
                    if mount_entry_param.is_empty() {
                        continue;
                    }

                    match index {
                        0 => src = Some(mount_entry_param),
                        1 => target = Some(mount_entry_param),
                        2 => fstype = Some(mount_entry_param),
                        3 => (flags, data) = serialized_flags_split(mount_entry_param)?,
                        4 => {
                            _dump = match mount_entry_param {
                                "0" => 0,
                                "1" => 1,
                                "2" => 2,
                                _ => 0,
                            }
                        }
                        5 => {
                            _fsck = match mount_entry_param {
                                "0" => 0,
                                "1" => 1,
                                "2" => 2,
                                _ => 0,
                            }
                        }
                        _ => return Err(libc::EINVAL),
                    };

                    index += 1;
                }

                if index < 3 {
                    return Err(libc::EINVAL);
                }

                let mount = Mountpoint::new(
                    src,
                    target.unwrap(),
                    fstype,
                    flags,
                    match data.empty() {
                        false => Some(data.as_slice().unwrap()),
                        true => None,
                    },
                )?;

                mounts.push(mount)?;
            }
        }

        Ok(Self { mounts })
    }

    pub fn iter_mounts(&'_ self) -> crate::vector::VecIter<'_, Mountpoint> {
        self.mounts.iter()
    }
}
