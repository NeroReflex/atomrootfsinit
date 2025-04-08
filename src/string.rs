pub struct CStr {
    alloc_sz: usize,
    data: *const libc::c_char,
}

impl Drop for CStr {
    fn drop(&mut self) {
        unsafe {
            if self.data != core::ptr::null_mut() {
                libc::free(self.data as *mut libc::c_void)
            }
        }
    }
}

impl CStr {
    pub fn new(str: &str) -> Result<Self, libc::c_int> {
        CStr::try_from(str.as_bytes())
    }

    pub fn strlen(&self) -> usize {
        /*
        for i in 0..self.alloc_sz {
            if unsafe { *self.data.offset(i as isize) } == 0 {
                return i;
            }
        }
        */

        self.alloc_sz - 1
    }

    pub fn inner(&self) -> *const libc::c_char {
        self.data
    }
}

pub(crate) fn search_in_slice<T>(slice: &[T], element: &T) -> Option<usize>
where
    T: PartialEq,
{
    for i in 0..slice.len() {
        if slice[i] == *element {
            return Some(i);
        }
    }

    None
}

impl TryFrom<&[u8]> for CStr {
    type Error = libc::c_int;

    fn try_from(str: &[u8]) -> Result<Self, Self::Error> {
        let true_str_len = search_in_slice(str, &('\0' as u8)).unwrap_or(str.len());

        let alloc_sz = true_str_len + 1;
        let data = unsafe { libc::malloc(alloc_sz) } as *mut libc::c_char;

        if data.is_null() {
            return Err(libc::ENOMEM);
        }

        unsafe {
            libc::memset(data as *mut libc::c_void, 0, alloc_sz);
            let _ = libc::memcpy(
                data as *mut libc::c_void,
                str.as_ptr() as *const libc::c_void,
                true_str_len,
            );
        }

        Ok(Self { alloc_sz, data })
    }
}
