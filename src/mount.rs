use crate::CStr;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MountFlag {
    Shared,
    Private,
    Slave,
    Unbindable,
    Recursive,
    DirSync,
    Lazytime,
    NoAccessTime,
    NoDev,
    NoExec,
    NoSUID,
    ReadOnly,
    RelativeAccessTime,
    Silent,
    Synchronous,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MountpointFlags {
    shared: bool,
    private: bool,
    slave: bool,
    unbindable: bool,
    recursive: bool,
    dirsync: bool,
    lazytime: bool,
    no_access_time: bool,
    no_dev: bool,
    no_exec: bool,
    no_suid: bool,
    read_only: bool,
    relative_access_time: bool,
    silent: bool,
    synchronous: bool,
}

impl MountpointFlags {
    pub fn new(flags: &[MountFlag]) -> Self {
        let mut mountpoint_flags = Self {
            shared: false,
            private: false,
            slave: false,
            unbindable: false,
            recursive: false,
            dirsync: false,
            lazytime: false,
            no_access_time: false,
            no_dev: false,
            no_exec: false,
            no_suid: false,
            read_only: false,
            relative_access_time: false,
            silent: false,
            synchronous: false,
        };

        for &flag in flags {
            match flag {
                MountFlag::Shared => mountpoint_flags.shared = true,
                MountFlag::Private => mountpoint_flags.private = true,
                MountFlag::Slave => mountpoint_flags.slave = true,
                MountFlag::Unbindable => mountpoint_flags.unbindable = true,
                MountFlag::Recursive => mountpoint_flags.recursive = true,
                MountFlag::DirSync => mountpoint_flags.dirsync = true,
                MountFlag::Lazytime => mountpoint_flags.lazytime = true,
                MountFlag::NoAccessTime => mountpoint_flags.no_access_time = true,
                MountFlag::NoDev => mountpoint_flags.no_dev = true,
                MountFlag::NoExec => mountpoint_flags.no_exec = true,
                MountFlag::NoSUID => mountpoint_flags.no_suid = true,
                MountFlag::ReadOnly => mountpoint_flags.read_only = true,
                MountFlag::RelativeAccessTime => mountpoint_flags.relative_access_time = true,
                MountFlag::Silent => mountpoint_flags.silent = true,
                MountFlag::Synchronous => mountpoint_flags.synchronous = true,
            }
        }

        mountpoint_flags
    }

    pub(crate) fn flags(&self) -> libc::c_ulong {
        (self.shared as libc::c_ulong * libc::MS_SHARED)
            | (self.private as libc::c_ulong * libc::MS_PRIVATE)
            | (self.slave as libc::c_ulong * libc::MS_SLAVE)
            | (self.unbindable as libc::c_ulong * libc::MS_UNBINDABLE)
            | (self.recursive as libc::c_ulong * libc::MS_REC)
            | (self.dirsync as libc::c_ulong * libc::MS_DIRSYNC)
            | (self.lazytime as libc::c_ulong * libc::MS_LAZYTIME)
            | (self.no_access_time as libc::c_ulong * libc::MS_NOATIME)
            | (self.no_dev as libc::c_ulong * libc::MS_NODEV)
            | (self.no_exec as libc::c_ulong * libc::MS_NOEXEC)
            | (self.no_suid as libc::c_ulong * libc::MS_NOSUID)
            | (self.read_only as libc::c_ulong * libc::MS_RDONLY)
            | (self.relative_access_time as libc::c_ulong * libc::MS_RELATIME)
            | (self.silent as libc::c_ulong * libc::MS_SILENT)
            | (self.synchronous as libc::c_ulong * libc::MS_SYNCHRONOUS)
    }
}

pub struct Mountpoint {
    src: Option<CStr>,
    target: CStr,
    fstype: Option<CStr>,
    data: *const libc::c_void,
    flags: MountpointFlags,
}

impl Drop for Mountpoint {
    fn drop(&mut self) {
        if self.data != core::ptr::null_mut() {
            unsafe { libc::free(self.data as *mut libc::c_void) }
        }
    }
}

impl Mountpoint {
    pub fn new(
        src: Option<&str>,
        target: &str,
        fstype: Option<&str>,
        flags: MountpointFlags,
        data: Option<&[u8]>,
    ) -> Result<Self, libc::c_int> {
        let src = match src {
            Some(str) => Some(CStr::new(str)?),
            None => None,
        };

        let target = CStr::new(target)?;

        let fstype = match fstype {
            Some(str) => Some(CStr::new(str)?),
            None => None,
        };

        let data = match data {
            Some(d) => unsafe {
                let data = libc::malloc(d.len()) as *mut libc::c_void;
                if data == core::ptr::null_mut() {
                    return Err(libc::ENOMEM);
                }

                libc::memcpy(data, d.as_ptr() as *const libc::c_void, d.len());

                data as *const libc::c_void
            },
            None => core::ptr::null(),
        };

        Ok(Self {
            src,
            target,
            fstype,
            data,
            flags,
        })
    }

    pub fn mount(&self) -> Result<(), libc::c_int> {
        let src = match &self.src {
            Some(ptr) => ptr.inner(),
            None => core::ptr::null() as *const libc::c_char,
        };

        let fstype = match &self.fstype {
            Some(ptr) => ptr.inner(),
            None => core::ptr::null() as *const libc::c_char,
        };

        unsafe {
            if libc::mount(
                src,
                self.target.inner(),
                fstype,
                self.flags.flags(),
                self.data,
            ) != 0
            {
                return Err(*libc::__errno_location());
            }
        }

        Ok(())
    }
}

pub fn direct_detach(target: &str) -> Result<(), libc::c_int> {
    let target_str = crate::CStr::new(target)?;

    unsafe {
        /*
         * On success, zero is returned.  On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::umount2(target_str.inner(), libc::MNT_DETACH) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
