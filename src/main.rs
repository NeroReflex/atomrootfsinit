#![no_main]

extern crate libc;

use atomrootfsinit::{
    config::Config,
    mount::{MountFlag, Mountpoint, MountpointFlags},
    string::CStr,
    switch_root::switch_root,
};

// Macro to print DEBUG messages only in debug builds
#[cfg(debug_assertions)]
macro_rules! debug_printf {
    ($($arg:tt)*) => {
        #[allow(unused_unsafe)] // unsafe is needed when macro is called from safe code
        unsafe {
            libc::printf($($arg)*);
        }
    };
}

#[cfg(not(debug_assertions))]
#[allow(unused_macros)]
macro_rules! debug_printf {
    ($($arg:tt)*) => {
        // No-op in release builds
    };
}

pub(crate) struct CmdLine {
    root: Option<CStr>,
    init: Option<CStr>,
}

fn read_partuuid_from_sys(
    sys_mount: &str,
    device_name: &str,
    needle: &str,
    #[allow(unused)] print_found: bool,
) -> bool {
    // Try {sys_mount}/class/block/{device}/uevent first
    let mut uevent_path = match atomrootfsinit::vector::Vec::<u8>::with_capacity(
        sys_mount.len() + 20 + device_name.len() + 7,
    ) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Build path: {sys_mount}/class/block/{device_name}/uevent
    for &b in sys_mount.as_bytes() {
        if uevent_path.push(b).is_err() {
            return false;
        }
    }
    for &b in b"/class/block/".iter() {
        if uevent_path.push(b).is_err() {
            return false;
        }
    }
    for b in device_name.bytes() {
        if uevent_path.push(b).is_err() {
            return false;
        }
    }
    for &b in b"/uevent".iter() {
        if uevent_path.push(b).is_err() {
            return false;
        }
    }

    let uevent_path_str = match uevent_path.as_slice() {
        Some(slice) => match core::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return false,
        },
        None => return false,
    };

    let uevent_content = match atomrootfsinit::read_whole_file(uevent_path_str, 512) {
        Ok(content) => content,
        Err(err) => {
            #[allow(unused)]
            let err = err;
            #[cfg(debug_assertions)]
            if print_found {
                let path_cstr =
                    CStr::new(uevent_path_str).unwrap_or_else(|_| CStr::new("").unwrap());
                let device_cstr =
                    CStr::new(device_name).unwrap_or_else(|_| CStr::new("").unwrap());
                debug_printf!(
                    b"Failed to read %s for device %s (errno %d), trying alternative path\n\0"
                        .as_ptr()
                        as *const libc::c_char,
                    path_cstr.inner(),
                    device_cstr.inner(),
                    err as libc::c_int,
                );
            }
            // Try alternative path: {sys_mount}/block/{disk}/{partition}/uevent
            // If device_name is like "sda1", try {sys_mount}/block/sda/sda1/uevent
            // Find last non-digit position
            let mut last_char_pos = device_name.len();
            for (i, ch) in device_name.char_indices().rev() {
                if !ch.is_ascii_digit() {
                    last_char_pos = i + 1;
                    break;
                }
            }

            if last_char_pos < device_name.len() {
                let disk = &device_name[..last_char_pos];
                let partition = device_name;

                let mut alt_path = match atomrootfsinit::vector::Vec::<u8>::with_capacity(
                    sys_mount.len() + 10 + disk.len() + 1 + partition.len() + 7,
                ) {
                    Ok(v) => v,
                    Err(_) => return false,
                };

                for &b in sys_mount.as_bytes() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }
                for &b in b"/block/".iter() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }
                for b in disk.bytes() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }
                for &b in b"/".iter() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }
                for b in partition.bytes() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }
                for &b in b"/uevent".iter() {
                    if alt_path.push(b).is_err() {
                        return false;
                    }
                }

                let alt_path_str = match alt_path.as_slice() {
                    Some(slice) => match core::str::from_utf8(slice) {
                        Ok(s) => s,
                        Err(_) => return false,
                    },
                    None => return false,
                };

                #[cfg(debug_assertions)]
                if print_found {
                    let alt_path_cstr =
                        CStr::new(alt_path_str).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Trying alternative path %s\n\0".as_ptr()
                            as *const libc::c_char,
                        alt_path_cstr.inner(),
                    );
                }

                match atomrootfsinit::read_whole_file(alt_path_str, 512) {
                    Ok(content) => content,
                    Err(err) => {
                        #[allow(unused)]
                        let err = err;
                        #[cfg(debug_assertions)]
                        if print_found {
                            let alt_path_cstr = CStr::new(alt_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            let device_cstr = CStr::new(device_name)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"Failed to read alternative path %s for device %s (errno %d)\n\0"
                                    .as_ptr()
                                    as *const libc::c_char,
                                alt_path_cstr.inner(),
                                device_cstr.inner(),
                                err as libc::c_int,
                            );
                        }
                        return false;
                    }
                }
            } else {
                #[cfg(debug_assertions)]
                if print_found {
                    let device_cstr =
                        CStr::new(device_name).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Device %s has no numeric suffix, cannot construct alternative path\n\0"
                            .as_ptr()
                            as *const libc::c_char,
                        device_cstr.inner(),
                    );
                }
                return false;
            }
        }
    };

    let uevent_str = match uevent_content.as_slice() {
        Some(slice) => match core::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return false,
        },
        None => return false,
    };

    for line in uevent_str.lines() {
        if let Some(rest) = line.strip_prefix("PARTUUID=") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                #[cfg(debug_assertions)]
                if print_found {
                    let partuuid_cstr =
                        CStr::new(trimmed).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Found PARTUUID %s on device %s\n\0".as_ptr()
                            as *const libc::c_char,
                        partuuid_cstr.inner(),
                    );
                    let device_cstr =
                        CStr::new(device_name).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Device: %s\n\0".as_ptr() as *const libc::c_char,
                        device_cstr.inner(),
                    );
                    let needle_cstr =
                        CStr::new(needle).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Looking for: %s\n\0".as_ptr() as *const libc::c_char,
                        needle_cstr.inner(),
                    );
                    let matches = if trimmed.eq_ignore_ascii_case(needle) {
                        "MATCH"
                    } else {
                        "NO MATCH"
                    };
                    let match_cstr =
                        CStr::new(matches).unwrap_or_else(|_| CStr::new("").unwrap());
                    debug_printf!(
                        b"Result: %s\n\0".as_ptr() as *const libc::c_char,
                        match_cstr.inner(),
                    );
                }
                if trimmed.eq_ignore_ascii_case(needle) {
                    return true;
                }
            }
        }
    }

    // No PARTUUID found - this is expected for most devices (only partitions have PARTUUIDs)
    // Don't log this as it creates too much noise

    false
}

fn is_block_device(dev_path: &str) -> bool {
    let path_cstr = match CStr::new(dev_path) {
        Ok(cstr) => cstr,
        Err(_) => {
            #[cfg(debug_assertions)]
            {
                let path_cstr = CStr::new(dev_path).unwrap_or_else(|_| CStr::new("").unwrap());
                debug_printf!(
                    b"Failed to create CStr for %s\n\0".as_ptr() as *const libc::c_char,
                    path_cstr.inner(),
                );
            }
            return false;
        }
    };

    let mut stat_buf: libc::stat = unsafe { core::mem::zeroed() };
    let result = unsafe { libc::stat(path_cstr.inner(), &mut stat_buf) };

    if result != 0 {
        #[cfg(debug_assertions)]
        {
            let path_cstr = CStr::new(dev_path).unwrap_or_else(|_| CStr::new("").unwrap());
            debug_printf!(
                b"stat failed for %s: errno %d\n\0".as_ptr() as *const libc::c_char,
                path_cstr.inner(),
                *libc::__errno_location(),
            );
        }
        return false;
    }

    // Check if it's a block device using S_ISBLK macro
    (stat_buf.st_mode & libc::S_IFMT) == libc::S_IFBLK
}

fn find_device_by_partuuid(
    needle: &str,
    sys_mount: &str,
    dev_mount: &str,
) -> Option<atomrootfsinit::vector::Vec<u8>> {
    #[cfg(debug_assertions)]
    {
        let needle_cstr = CStr::new(needle).unwrap_or_else(|_| CStr::new("").unwrap());
        debug_printf!(
            b"\nSearching for PARTUUID: %s\n\0".as_ptr() as *const libc::c_char,
            needle_cstr.inner(),
        );
        let sys_cstr = CStr::new(sys_mount).unwrap_or_else(|_| CStr::new("").unwrap());
        debug_printf!(
            b"sysfs mounted at: %s\n\0".as_ptr() as *const libc::c_char,
            sys_cstr.inner(),
        );
        let dev_cstr = CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
        debug_printf!(
            b"devtmpfs mounted at: %s\n\0".as_ptr() as *const libc::c_char,
            dev_cstr.inner(),
        );
    }

    // Try {sys_mount}/class/block first (or whatever sys is mounted at)
    if let Ok(mut sys_block_path) =
        atomrootfsinit::vector::Vec::<u8>::with_capacity(sys_mount.len() + 14)
    {
        for &b in sys_mount.as_bytes() {
            if sys_block_path.push(b).is_err() {
                break;
            }
        }
        for &b in b"/class/block".iter() {
            if sys_block_path.push(b).is_err() {
                break;
            }
        }

        if let Some(sys_block_path_slice) = sys_block_path.as_slice() {
            if let Ok(sys_block_path_str) = core::str::from_utf8(sys_block_path_slice) {
                // First, check if sysfs root exists and what's in it
                let sys_root_cstr = match CStr::new(sys_mount) {
                    Ok(cstr) => cstr,
                    Err(_) => {
                        #[cfg(debug_assertions)]
                        debug_printf!(
                            b"Failed to create CStr for sys_mount root\n\0".as_ptr()
                                as *const libc::c_char,
                        );
                        // Continue to try opening class/block anyway
                        return None;
                    }
                };

                let sys_root_dir = unsafe { libc::opendir(sys_root_cstr.inner()) };
                if !sys_root_dir.is_null() {
                    #[cfg(debug_assertions)]
                    #[cfg(debug_assertions)]
                    {
                        debug_printf!(
                            b"Contents of %s:\n\0".as_ptr() as *const libc::c_char,
                            sys_root_cstr.inner(),
                        );
                        let mut entry_count = 0;
                        loop {
                            let entry = unsafe { libc::readdir(sys_root_dir) };
                            if entry.is_null() {
                                break;
                            }
                            let d_name = unsafe { (*entry).d_name.as_ptr() };
                            let mut name_len = 0;
                            while unsafe { *d_name.add(name_len) } != 0 {
                                name_len += 1;
                            }
                            if name_len == 0 {
                                continue;
                            }
                            let first_char = unsafe { *d_name } as u8 as char;
                            if first_char == '.'
                                && (name_len == 1 || unsafe { *d_name.add(1) } as u8 as char == '.')
                            {
                                continue;
                            }
                            entry_count += 1;
                            let name_bytes =
                                unsafe { core::slice::from_raw_parts(d_name as *const u8, name_len) };
                            if let Ok(name_str) = core::str::from_utf8(name_bytes) {
                                let name_cstr =
                                    CStr::new(name_str).unwrap_or_else(|_| CStr::new("").unwrap());
                                debug_printf!(
                                    b"%s\n\0".as_ptr() as *const libc::c_char,
                                    name_cstr.inner(),
                                );
                            }
                        }
                        unsafe {
                            libc::closedir(sys_root_dir);
                        }
                        debug_printf!(
                            b"Found %d entries in %s\n\0".as_ptr() as *const libc::c_char,
                            entry_count,
                            sys_root_cstr.inner(),
                        );
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        // In release, we still need to close the directory, but don't need to count entries
                        loop {
                            let entry = unsafe { libc::readdir(sys_root_dir) };
                            if entry.is_null() {
                                break;
                            }
                        }
                        unsafe {
                            libc::closedir(sys_root_dir);
                        }
                    }
                } else {
                    #[cfg(debug_assertions)]
                    debug_printf!(
                        b"Failed to open sysfs root %s, errno %d\n\0".as_ptr()
                            as *const libc::c_char,
                        sys_root_cstr.inner(),
                        *libc::__errno_location(),
                    );
                }

                if let Ok(sys_block_cstr) = CStr::new(sys_block_path_str) {
                    #[cfg(debug_assertions)]
                    {
                        let path_cstr = CStr::new(sys_block_path_str)
                            .unwrap_or_else(|_| CStr::new("").unwrap());
                        debug_printf!(
                            b"Attempting to open %s\n\0".as_ptr() as *const libc::c_char,
                            path_cstr.inner(),
                        );
                    }

                    let dir = unsafe { libc::opendir(sys_block_cstr.inner()) };

                    if !dir.is_null() {
                        #[cfg(debug_assertions)]
                        debug_printf!(
                            b"Successfully opened %s\n\0".as_ptr()
                                as *const libc::c_char,
                            CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap())
                                .inner(),
                        );
                        let mut result: Option<atomrootfsinit::vector::Vec<u8>> = None;
                        let mut device_count = 0;

                        loop {
                            let entry = unsafe { libc::readdir(dir) };
                            if entry.is_null() {
                                break;
                            }

                            let d_name = unsafe { (*entry).d_name.as_ptr() };
                            let mut name_len = 0;
                            while unsafe { *d_name.add(name_len) } != 0 {
                                name_len += 1;
                            }

                            if name_len == 0 {
                                continue;
                            }

                            // Skip . and ..
                            let first_char = unsafe { *d_name } as u8 as char;
                            if first_char == '.'
                                && (name_len == 1 || unsafe { *d_name.add(1) } as u8 as char == '.')
                            {
                                continue;
                            }

                            device_count += 1;

                            // Read device name
                            let device_name_bytes = unsafe {
                                core::slice::from_raw_parts(d_name as *const u8, name_len)
                            };
                            let device_name = match core::str::from_utf8(device_name_bytes) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };

                            #[cfg(debug_assertions)]
                            {
                                let device_cstr = CStr::new(device_name)
                                    .unwrap_or_else(|_| CStr::new("").unwrap());
                                debug_printf!(
                                    b"Checking device %s from /class/block\n\0".as_ptr()
                                        as *const libc::c_char,
                                    device_cstr.inner(),
                                );
                            }

                            // Read PARTUUID from sys and compare
                            if read_partuuid_from_sys(sys_mount, device_name, needle, true) {
                                // Store device name as Vec<u8>
                                let mut device_name_vec =
                                    match atomrootfsinit::vector::Vec::<u8>::with_capacity(
                                        device_name.len(),
                                    ) {
                                        Ok(v) => v,
                                        Err(_) => break,
                                    };
                                for b in device_name.bytes() {
                                    if device_name_vec.push(b).is_err() {
                                        break;
                                    }
                                }
                                #[cfg(debug_assertions)]
                                {
                                    let device_cstr = CStr::new(device_name)
                                        .unwrap_or_else(|_| CStr::new("").unwrap());
                                    debug_printf!(
                                        b"MATCH FOUND! Device: %s\n\0".as_ptr()
                                            as *const libc::c_char,
                                        device_cstr.inner(),
                                    );
                                }
                                result = Some(device_name_vec);
                                break;
                            }
                        }

                        unsafe {
                            libc::closedir(dir);
                        }
                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"Scanned %d devices in %s\n\0".as_ptr()
                                    as *const libc::c_char,
                                device_count,
                                path_cstr.inner(),
                            );
                        }
                        if device_count == 0 {
                            unsafe {
                                libc::printf(
                                    b"ERROR: /class/block directory is empty! No block devices found in sysfs.\n\0"
                                        .as_ptr()
                                        as *const libc::c_char,
                                );
                            }
                        }

                        if result.is_some() {
                            return result;
                        }

                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            let dev_cstr =
                                CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"No match found in %s (directory was empty or no matching PARTUUID), trying %s scan\n\0".as_ptr()
                                    as *const libc::c_char,
                                path_cstr.inner(),
                                dev_cstr.inner(),
                            );
                        }
                    } else {
                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            let dev_cstr =
                                CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"Failed to open %s (errno %d), trying %s scan\n\0".as_ptr()
                                    as *const libc::c_char,
                                path_cstr.inner(),
                                *libc::__errno_location(),
                                dev_cstr.inner(),
                            );
                        }

                        // Try /block as alternative (some initramfs systems might not have /class/block)
                        if let Ok(mut block_path) =
                            atomrootfsinit::vector::Vec::<u8>::with_capacity(
                                sys_mount.len() + 7,
                            )
                        {
                            for &b in sys_mount.as_bytes() {
                                if block_path.push(b).is_err() {
                                    break;
                                }
                            }
                            for &b in b"/block".iter() {
                                if block_path.push(b).is_err() {
                                    break;
                                }
                            }

                            if let Some(block_path_slice) = block_path.as_slice() {
                                if let Ok(block_path_str) =
                                    core::str::from_utf8(block_path_slice)
                                {
                                    let block_path_cstr = CStr::new(block_path_str);
                                    if let Ok(block_cstr) = block_path_cstr {
                                        let block_dir = unsafe { libc::opendir(block_cstr.inner()) };
                                        if !block_dir.is_null() {
                                            #[cfg(debug_assertions)]
                                            {
                                                let block_path_cstr_for_print =
                                                    CStr::new(block_path_str)
                                                        .unwrap_or_else(|_| CStr::new("").unwrap());
                                                debug_printf!(
                                                    b"Found alternative %s directory, will try that in fallback\n\0".as_ptr()
                                                        as *const libc::c_char,
                                                    block_path_cstr_for_print.inner(),
                                                );
                                            }
                                            unsafe {
                                                libc::closedir(block_dir);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Alternative: try {sys_mount}/block directly (kernel creates /sys/block for block devices)
    if let Ok(mut sys_block_path) =
        atomrootfsinit::vector::Vec::<u8>::with_capacity(sys_mount.len() + 7)
    {
        for &b in sys_mount.as_bytes() {
            if sys_block_path.push(b).is_err() {
                break;
            }
        }
        for &b in b"/block".iter() {
            if sys_block_path.push(b).is_err() {
                break;
            }
        }

        if let Some(sys_block_path_slice) = sys_block_path.as_slice() {
            if let Ok(sys_block_path_str) = core::str::from_utf8(sys_block_path_slice) {
                if let Ok(sys_block_cstr) = CStr::new(sys_block_path_str) {
                    #[cfg(debug_assertions)]
                    {
                        let path_cstr = CStr::new(sys_block_path_str)
                            .unwrap_or_else(|_| CStr::new("").unwrap());
                        debug_printf!(
                            b"Trying alternative path %s\n\0".as_ptr()
                                as *const libc::c_char,
                            path_cstr.inner(),
                        );
                    }

                    let dir = unsafe { libc::opendir(sys_block_cstr.inner()) };

                    if !dir.is_null() {
                        #[cfg(debug_assertions)]
                        debug_printf!(
                            b"Successfully opened %s\n\0".as_ptr()
                                as *const libc::c_char,
                            CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap())
                                .inner(),
                        );
                        let mut result: Option<atomrootfsinit::vector::Vec<u8>> = None;
                        let mut device_count = 0;

                        loop {
                            let entry = unsafe { libc::readdir(dir) };
                            if entry.is_null() {
                                break;
                            }

                            let d_name = unsafe { (*entry).d_name.as_ptr() };
                            let mut name_len = 0;
                            while unsafe { *d_name.add(name_len) } != 0 {
                                name_len += 1;
                            }

                            if name_len == 0 {
                                continue;
                            }

                            // Skip . and ..
                            let first_char = unsafe { *d_name } as u8 as char;
                            if first_char == '.'
                                && (name_len == 1 || unsafe { *d_name.add(1) } as u8 as char == '.')
                            {
                                continue;
                            }

                            device_count += 1;

                            // Read device name
                            let device_name_bytes = unsafe {
                                core::slice::from_raw_parts(d_name as *const u8, name_len)
                            };
                            let device_name = match core::str::from_utf8(device_name_bytes) {
                                Ok(s) => s,
                                Err(_) => continue,
                            };

                            #[cfg(debug_assertions)]
                            {
                                let device_cstr = CStr::new(device_name)
                                    .unwrap_or_else(|_| CStr::new("").unwrap());
                                debug_printf!(
                                    b"Checking device %s from /block\n\0".as_ptr()
                                        as *const libc::c_char,
                                    device_cstr.inner(),
                                );
                            }

                            // Read PARTUUID from sys and compare
                            if read_partuuid_from_sys(sys_mount, device_name, needle, true) {
                                // Store device name as Vec<u8>
                                let mut device_name_vec =
                                    match atomrootfsinit::vector::Vec::<u8>::with_capacity(
                                        device_name.len(),
                                    ) {
                                        Ok(v) => v,
                                        Err(_) => break,
                                    };
                                for b in device_name.bytes() {
                                    if device_name_vec.push(b).is_err() {
                                        break;
                                    }
                                }
                                #[cfg(debug_assertions)]
                                {
                                    let device_cstr = CStr::new(device_name)
                                        .unwrap_or_else(|_| CStr::new("").unwrap());
                                    debug_printf!(
                                        b"MATCH FOUND! Device: %s\n\0".as_ptr()
                                            as *const libc::c_char,
                                        device_cstr.inner(),
                                    );
                                }
                                result = Some(device_name_vec);
                                break;
                            }
                        }

                        unsafe {
                            libc::closedir(dir);
                        }
                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"Scanned %d devices in %s\n\0".as_ptr()
                                    as *const libc::c_char,
                                device_count,
                                path_cstr.inner(),
                            );
                        }
                        if device_count == 0 {
                            unsafe {
                                libc::printf(
                                    b"ERROR: /block directory is empty! No block devices found in sysfs.\n\0"
                                        .as_ptr()
                                        as *const libc::c_char,
                                );
                            }
                        }

                        if result.is_some() {
                            return result;
                        }

                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            let dev_cstr =
                                CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"No match found in %s, trying %s scan\n\0".as_ptr()
                                    as *const libc::c_char,
                                path_cstr.inner(),
                                dev_cstr.inner(),
                            );
                        }
                    } else {
                        #[cfg(debug_assertions)]
                        {
                            let path_cstr = CStr::new(sys_block_path_str)
                                .unwrap_or_else(|_| CStr::new("").unwrap());
                            let dev_cstr =
                                CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
                            debug_printf!(
                                b"Failed to open %s (errno %d), trying %s scan\n\0".as_ptr()
                                    as *const libc::c_char,
                                path_cstr.inner(),
                                *libc::__errno_location(),
                                dev_cstr.inner(),
                            );
                        }
                    }
                }
            }
        }
    }

    // Fallback: scan {dev_mount} for block devices
    #[cfg(debug_assertions)]
    {
        let dev_cstr = CStr::new(dev_mount).unwrap_or_else(|_| CStr::new("").unwrap());
        debug_printf!(
            b"Scanning %s for block devices\n\0".as_ptr() as *const libc::c_char,
            dev_cstr.inner(),
        );
    }

    let dev_mount_cstr = match CStr::new(dev_mount) {
        Ok(cstr) => cstr,
        Err(_) => {
            #[cfg(debug_assertions)]
            debug_printf!(b"Failed to create CStr for dev_mount\n\0".as_ptr()
                as *const libc::c_char);
            return None;
        }
    };

    let dev_dir = unsafe { libc::opendir(dev_mount_cstr.inner()) };
    if dev_dir.is_null() {
        #[cfg(debug_assertions)]
        debug_printf!(
            b"Failed to open dev directory, errno %d\n\0".as_ptr()
                as *const libc::c_char,
            *libc::__errno_location(),
        );
        return None;
    }

    let mut result: Option<atomrootfsinit::vector::Vec<u8>> = None;
    #[cfg(debug_assertions)]
    let mut checked_count = 0;
    #[cfg(debug_assertions)]
    let mut block_count = 0;

    loop {
        let entry = unsafe { libc::readdir(dev_dir) };
        if entry.is_null() {
            break;
        }

        let d_name = unsafe { (*entry).d_name.as_ptr() };
        let mut name_len = 0;
        while unsafe { *d_name.add(name_len) } != 0 {
            name_len += 1;
        }

        if name_len == 0 {
            continue;
        }

        // Skip . and ..
        let first_char = unsafe { *d_name } as u8 as char;
        if first_char == '.' && (name_len == 1 || unsafe { *d_name.add(1) } as u8 as char == '.') {
            continue;
        }

        // Read device name
        let device_name_bytes =
            unsafe { core::slice::from_raw_parts(d_name as *const u8, name_len) };
        let device_name = match core::str::from_utf8(device_name_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        #[cfg(debug_assertions)]
        {
            checked_count += 1;
        }

        // Build full path in {dev_mount}
        let dev_mount_str = dev_mount_cstr.as_str();
        let mut full_path = match atomrootfsinit::vector::Vec::<u8>::with_capacity(
            dev_mount_str.len() + 1 + device_name.len(),
        ) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for &b in dev_mount_str.as_bytes() {
            if full_path.push(b).is_err() {
                continue;
            }
        }
        if !dev_mount_str.ends_with('/') {
            if full_path.push(b'/').is_err() {
                continue;
            }
        }
        for b in device_name.bytes() {
            if full_path.push(b).is_err() {
                continue;
            }
        }

        let full_path_str = match full_path.as_slice() {
            Some(slice) => match core::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => continue,
            },
            None => continue,
        };

        // Check if it's a block device
        if !is_block_device(full_path_str)
            || full_path_str.contains("ram")
            || full_path_str.contains("loop")
        {
            continue;
        }

        #[cfg(debug_assertions)]
        {
            block_count += 1;
        }

        #[cfg(debug_assertions)]
        {
            let device_cstr = CStr::new(device_name).unwrap_or_else(|_| CStr::new("").unwrap());
            debug_printf!(
                b"Checking block device %s from /dev scan\n\0".as_ptr()
                    as *const libc::c_char,
                device_cstr.inner(),
            );
        }

        // Try to read PARTUUID from sys and compare
        if read_partuuid_from_sys(sys_mount, device_name, needle, true) {
            // Store device name as Vec<u8>
            let mut device_name_vec =
                match atomrootfsinit::vector::Vec::<u8>::with_capacity(device_name.len()) {
                    Ok(v) => v,
                    Err(_) => break,
                };
            for b in device_name.bytes() {
                if device_name_vec.push(b).is_err() {
                    break;
                }
            }
            #[cfg(debug_assertions)]
            {
                let device_cstr = CStr::new(device_name).unwrap_or_else(|_| CStr::new("").unwrap());
                debug_printf!(
                    b"MATCH FOUND! Device: %s\n\0".as_ptr() as *const libc::c_char,
                    device_cstr.inner(),
                );
            }
            result = Some(device_name_vec);
            break;
        }
    }

    unsafe {
        libc::closedir(dev_dir);
    }
    #[cfg(debug_assertions)]
    {
        debug_printf!(
            b"Scanned %d entries, found %d block devices\n\0".as_ptr()
                as *const libc::c_char,
            checked_count,
            block_count,
        );
        if result.is_none() {
            debug_printf!(
                b"No matching device found for PARTUUID %s\n\0".as_ptr()
                    as *const libc::c_char,
                CStr::new(needle)
                    .unwrap_or_else(|_| CStr::new("").unwrap())
                    .inner(),
            );
        }
    }

    result
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
                                    b"pop %02x\n\0".as_ptr() as *const libc::c_char,
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

    // First pass: find sysfs, devtmpfs, and rootdev mounts
    let mut sys_mount_point: Option<&str> = None;
    let mut dev_mount_point: Option<&str> = None;
    let mut rootdev_mount: Option<&Mountpoint> = None;
    let mut rootfs_target = atomrootfsinit::SYSROOT;

    for mount in config.iter_mounts() {
        // Track sysfs and devtmpfs mount points
        if let Some(fstype) = mount.fstype() {
            if fstype == "sysfs" {
                sys_mount_point = Some(mount.target());
            } else if fstype == "devtmpfs" {
                dev_mount_point = Some(mount.target());
            }
        }
        // Track rootdev mount for later processing
        if let Some(src) = mount.src() {
            if src == "rootdev" {
                rootdev_mount = Some(mount);
                rootfs_target = mount.target();
            }
        }
    }

    // Mount sysfs and devtmpfs first if they exist (needed for PARTUUID resolution)
    for mount in config.iter_mounts() {
        if let Some(fstype) = mount.fstype() {
            if fstype == "sysfs" || fstype == "devtmpfs" {
                #[cfg(feature = "trace")]
                unsafe {
                    libc::printf(
                        b"Mounting %s\n\0".as_ptr() as *const libc::c_char,
                        mount.target(),
                    );
                }

                if let Err(err) = mount.mount(&None) {
                    unsafe {
                        libc::printf(
                            b"Failed to mount %s: %d\n\0".as_ptr() as *const libc::c_char,
                            mount.target().as_ptr() as *const libc::c_char,
                            err as libc::c_int,
                        );
                    }
                    return exit_error(err);
                }
            }
        }
    }

    // Now resolve PARTUUID for rootdev if needed (sysfs should be mounted now)
    let mut resolved_rootdev: Option<CStr> = None;
    if let Some(_mount) = rootdev_mount {
        let rootfs = cmdline.as_ref().map_or(None, |a| a.root.clone());

        if let Some(ref rootfs_str) = rootfs {
            let rootfs_val = rootfs_str.as_str();
            if rootfs_val.starts_with("PARTUUID=") {
                let partuuid = &rootfs_val[9..]; // Skip "PARTUUID="

                // Find device in {sys_mount}/class/block
                // Use tracked mount points or fallback to defaults
                let sys_mount = sys_mount_point.unwrap_or("/sys");
                let dev_mount = dev_mount_point.unwrap_or("/dev");

                if let Some(device_name_bytes) =
                    find_device_by_partuuid(partuuid, sys_mount, dev_mount)
                {
                    // Build device path using the devtmpfs mount point
                    let device_name_slice = match device_name_bytes.as_slice() {
                        Some(s) => s,
                        None => {
                            unsafe {
                                libc::printf(b"Failed to get device name slice\n\0".as_ptr()
                                    as *const libc::c_char);
                            }
                            return exit_error(libc::EINVAL);
                        }
                    };

                    // Build device path manually
                    let prefix_bytes = if dev_mount == "/dev" {
                        b"/dev/"
                    } else {
                        dev_mount.as_bytes()
                    };

                    let mut device_path_vec = match atomrootfsinit::vector::Vec::<u8>::with_capacity(
                        prefix_bytes.len() + 1 + device_name_slice.len(),
                    ) {
                        Ok(v) => v,
                        Err(err) => {
                            unsafe {
                                libc::printf(
                                    b"Failed to allocate device path: %d\n\0".as_ptr()
                                        as *const libc::c_char,
                                    err as libc::c_int,
                                );
                            }
                            return exit_error(err);
                        }
                    };

                    // Copy prefix
                    for &b in prefix_bytes {
                        if device_path_vec.push(b).is_err() {
                            unsafe {
                                libc::printf(b"Failed to build device path\n\0".as_ptr()
                                    as *const libc::c_char);
                            }
                            return exit_error(libc::ENOMEM);
                        }
                    }

                    // Add trailing slash if not /dev/ (and mount point doesn't end with /)
                    if dev_mount != "/dev" && !dev_mount.ends_with('/') {
                        if device_path_vec.push(b'/').is_err() {
                            unsafe {
                                libc::printf(b"Failed to build device path\n\0".as_ptr()
                                    as *const libc::c_char);
                            }
                            return exit_error(libc::ENOMEM);
                        }
                    }

                    // Copy device name
                    for &b in device_name_slice {
                        if device_path_vec.push(b).is_err() {
                            unsafe {
                                libc::printf(b"Failed to build device path\n\0".as_ptr()
                                    as *const libc::c_char);
                            }
                            return exit_error(libc::ENOMEM);
                        }
                    }

                    let device_path_slice = match device_path_vec.as_slice() {
                        Some(s) => s,
                        None => {
                            unsafe {
                                libc::printf(b"Failed to get device path slice\n\0".as_ptr()
                                    as *const libc::c_char);
                            }
                            return exit_error(libc::EINVAL);
                        }
                    };

                    let device_path_str = match core::str::from_utf8(device_path_slice) {
                        Ok(s) => s,
                        Err(_) => {
                            unsafe {
                                libc::printf(
                                    b"Failed to convert device path to UTF-8\n\0".as_ptr()
                                        as *const libc::c_char,
                                );
                            }
                            return exit_error(libc::EINVAL);
                        }
                    };

                    resolved_rootdev = Some(CStr::new(device_path_str).unwrap_or_else(|err| {
                        unsafe {
                            libc::printf(
                                b"Failed to allocate device path for PARTUUID: %d\n\0".as_ptr()
                                    as *const libc::c_char,
                                err as libc::c_int,
                            );
                        }
                        exit_error(err);
                        unreachable!()
                    }));
                } else {
                    // Create CStr for printf
                    let partuuid_cstr = CStr::new(partuuid).unwrap_or_else(|err| {
                        unsafe {
                            libc::printf(
                                b"Failed to allocate PARTUUID string for error message: %d\n\0"
                                    .as_ptr()
                                    as *const libc::c_char,
                                err as libc::c_int,
                            );
                        }
                        exit_error(err);
                        unreachable!()
                    });
                    unsafe {
                        libc::printf(
                            b"Failed to find device with PARTUUID %s\n\0".as_ptr()
                                as *const libc::c_char,
                            partuuid_cstr.inner(),
                        );
                    }
                    return exit_error(libc::ENODEV);
                }
            } else {
                // No PARTUUID, use rootfs as-is
                resolved_rootdev = rootfs;
            }
        } else {
            // No rootfs from cmdline
            resolved_rootdev = None;
        }
    }

    // Now mount all other mounts (including rootdev if it wasn't already processed)
    for mount in config.iter_mounts() {
        // Skip sysfs and devtmpfs - already mounted
        if let Some(fstype) = mount.fstype() {
            if fstype == "sysfs" || fstype == "devtmpfs" {
                continue;
            }
        }

        let rootfs = if mount.src().map(|s| s == "rootdev").unwrap_or(false) {
            &resolved_rootdev
        } else {
            &None
        };

        #[cfg(feature = "trace")]
        unsafe {
            libc::printf(
                b"Mounting %s\n\0".as_ptr() as *const libc::c_char,
                mount.target(),
            );
        }

        if let Err(err) = mount.mount(&rootfs) {
            match &rootfs {
                Some(rootfs) => unsafe {
                    libc::printf(
                        b"Failed to mount %s from %s: %d\n\0".as_ptr() as *const libc::c_char,
                        mount.target().as_ptr() as *const libc::c_char,
                        rootfs.as_str().as_ptr() as *const libc::c_char,
                        err as libc::c_int,
                    );
                },
                None => unsafe {
                    libc::printf(
                        b"Failed to mount %s: %d\n\0".as_ptr() as *const libc::c_char,
                        mount.target().as_ptr() as *const libc::c_char,
                        err as libc::c_int,
                    );
                },
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
