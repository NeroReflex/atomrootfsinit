use crate::mount::{MountFlag, Mountpoint, MountpointFlags};
use crate::vector::Vec;

pub struct Config {
    mounts: Vec<Mountpoint>,
}

impl Config {
    pub fn new(content: Vec<u8>) -> Result<Self, libc::c_int> {
        let mut mounts = Vec::<Mountpoint>::default();

        let raw_data = content.split('\n' as u8, false)?;
        drop(content);

        for mount_entry_line in raw_data.iter() {
            let mount_entry_params = mount_entry_line.split(' ' as u8, false)?;

            let target = match mount_entry_params.at(0) {
                Some(str) => Vec::<u8>::new(str.as_slice())?,
                None => return Err(libc::EINVAL),
            };

            let src = match mount_entry_params.at(1) {
                Some(str) => Vec::<u8>::new(str.as_slice())?,
                None => return Err(libc::EINVAL),
            };

            let fstype = match mount_entry_params.at(2) {
                Some(str) => Vec::<u8>::new(str.as_slice())?,
                None => return Err(libc::EINVAL),
            };

            let (flags, data) = match mount_entry_params.at(3) {
                Some(str) => {
                    let serialized_flags =
                        core::str::from_utf8(str.as_slice()).map_err(|_| libc::EINVAL)?;

                    let mut flags = Vec::<MountFlag>::default();
                    let mut data = Vec::<u8>::default();

                    for flag in serialized_flags.split(',').into_iter() {
                        match flag {
                            "rw" => {}
                            "noatime" => flags.push(MountFlag::NoAccessTime)?,
                            "remount" => flags.push(MountFlag::Remount)?,
                            "recursive" => flags.push(MountFlag::Recursive)?,
                            "bind" => flags.push(MountFlag::Bind)?,
                            "ro" => flags.push(MountFlag::ReadOnly)?,
                            flg => {
                                for d in flg.as_bytes().into_iter() {
                                    if !data.empty() {
                                        data.push(',' as u8)?;
                                    }

                                    data.push(*d)?;
                                }
                            }
                        }
                    }

                    // mount flags are given as C string to the kernel: ensure it is NULL-terminated
                    if !data.empty() {
                        data.push(0u8)?;
                    }

                    (MountpointFlags::new(flags.as_slice()), data)
                }
                None => (MountpointFlags::new(&[]), Vec::<u8>::default()),
            };

            let mount = Mountpoint::new(
                Some(src.as_slice()),
                target.as_slice(),
                Some(fstype.as_slice()),
                flags,
                Some(data.as_slice()),
            )?;

            mounts.push(mount)?;
        }

        Ok(Self { mounts })
    }

    pub fn iter_mounts(&self) -> crate::vector::VecIter<Mountpoint> {
        self.mounts.iter()
    }
}
