use core::ptr;

use crate::string::CStr;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MountFlag {
    Bind,
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
    Remount,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct MountpointFlags {
    bind: bool,
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
    remount: bool,
}

impl MountpointFlags {
    pub fn new(flags: &[MountFlag]) -> Self {
        let mut mountpoint_flags = Self {
            bind: false,
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
            remount: false,
        };

        for &flag in flags {
            match flag {
                MountFlag::Bind => mountpoint_flags.bind = true,
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
                MountFlag::Remount => mountpoint_flags.remount = true,
            }
        }

        mountpoint_flags
    }

    pub fn set(&mut self, flag: MountFlag) {
        match flag {
            MountFlag::Bind => self.bind = true,
            MountFlag::Shared => self.shared = true,
            MountFlag::Private => self.private = true,
            MountFlag::Slave => self.slave = true,
            MountFlag::Unbindable => self.unbindable = true,
            MountFlag::Recursive => self.recursive = true,
            MountFlag::DirSync => self.dirsync = true,
            MountFlag::Lazytime => self.lazytime = true,
            MountFlag::NoAccessTime => self.no_access_time = true,
            MountFlag::NoDev => self.no_dev = true,
            MountFlag::NoExec => self.no_exec = true,
            MountFlag::NoSUID => self.no_suid = true,
            MountFlag::ReadOnly => self.read_only = true,
            MountFlag::RelativeAccessTime => self.relative_access_time = true,
            MountFlag::Silent => self.silent = true,
            MountFlag::Synchronous => self.synchronous = true,
            MountFlag::Remount => self.remount = true,
        }
    }

    pub(crate) fn flags(&self) -> libc::c_ulong {
        (self.bind as libc::c_ulong * libc::MS_BIND)
            | (self.shared as libc::c_ulong * libc::MS_SHARED)
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
            | (self.remount as libc::c_ulong * libc::MS_REMOUNT)
    }
}

#[derive(Debug)]
pub struct Mountpoint {
    src: Option<CStr>,
    target: CStr,
    fstype: Option<CStr>,
    data: *const libc::c_void,
    data_len: usize,
    flags: MountpointFlags,
}

impl Drop for Mountpoint {
    fn drop(&mut self) {
        if !self.data.is_null() {
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

        let (data, data_len) = match data {
            Some(d) => unsafe {
                match d.is_empty() {
                    false => {
                        let data = libc::malloc(d.len()) as *mut libc::c_void;
                        if data.is_null() {
                            return Err(libc::ENOMEM);
                        }
                        ptr::copy(d.as_ptr() as *const libc::c_void, data, d.len());

                        (data as *const libc::c_void, d.len())
                    }
                    true => (ptr::null(), 0),
                }
            },
            None => (core::ptr::null(), 0),
        };

        Ok(Self {
            src,
            target,
            fstype,
            data,
            data_len,
            flags,
        })
    }

    pub fn mount(&self, rootdev: &Option<CStr>) -> Result<(), libc::c_int> {
        let src = match &self.src {
            Some(ptr) => match ptr.as_str() {
                "rootdev" => match &rootdev {
                    Some(rd) => rd.inner(),
                    None => core::ptr::null() as *const libc::c_char,
                },
                _ => ptr.inner(),
            },
            None => core::ptr::null() as *const libc::c_char,
        };

        let fstype = match &self.fstype {
            Some(ptr) => ptr.inner(),
            None => core::ptr::null() as *const libc::c_char,
        };

        unsafe {
            // Example:
            // mount("overlay", "/merged", "overlay", 0, "lowerdir=/etc,upperdir=/upper,wo"...)
            // mount("src", "target", "<ignored>", MS_BIND, NULL)
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

    pub fn target(&self) -> &str {
        let slice = unsafe {
            core::slice::from_raw_parts(self.target.inner() as *const u8, self.target.strlen())
        };

        unsafe { core::str::from_utf8_unchecked(slice) }
    }

    pub fn src(&self) -> Option<&str> {
        match &self.src {
            Some(src) => Some(unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    src.inner() as *const u8,
                    src.strlen(),
                ))
            }),
            None => None,
        }
    }

    pub fn data(&self) -> Option<&[u8]> {
        let data_ptr = self.data as *const u8;
        if data_ptr.is_null() || self.data_len == 0 {
            return None;
        }

        Some(unsafe { core::slice::from_raw_parts(data_ptr, self.data_len) })
    }
}

pub fn direct_detach(target: &str) -> Result<(), libc::c_int> {
    let target_str = CStr::new(target)?;

    unsafe {
        /*
         * On success, zero is returned. On error, -1 is returned, and errno
         * is set to indicate the error.
         */
        if libc::umount2(target_str.inner(), libc::MNT_DETACH) != 0 {
            return Err(*libc::__errno_location());
        }
    }

    Ok(())
}
