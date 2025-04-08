use crate::vector::Vec;
use crate::string::CStr;
use crate::mount::{MountFlag, Mountpoint, MountpointFlags};

const BUFFER_SIZE: usize = 8192;

pub struct Config {
    mounts: Vec<Mountpoint>,
}

impl Config {
    pub fn new(path: &str) -> Result<Self, libc::c_int> {
        let mut content = Vec::<u8>::with_capacity(BUFFER_SIZE)?;

        let path_str = CStr::new(path)?;

        unsafe {
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
        }

        let mut mounts = Vec::<Mountpoint>::default();

        let raw_data = content.split('\n' as u8, false)?;
        drop(content);

        for i in 0..raw_data.len() {
            let mount_entry_params = match raw_data.at(i) {
                Some(e) => e.split(' ' as u8, false)?,
                None => unreachable!(),
            };

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

                    for flag in serialized_flags.split(' ').into_iter() {
                        match flag {
                            "rw" => {}
                            "ro" => {
                                flags.push(MountFlag::ReadOnly)?;
                            }
                            flg => {
                                for d in flg.as_bytes().into_iter() {
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
