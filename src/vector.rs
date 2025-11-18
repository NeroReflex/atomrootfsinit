extern crate libc;

use core::mem;
use core::ptr;

pub struct Vec<T> {
    ptr: *mut T,
    capacity: usize,
    length: usize,
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self {
            ptr: ptr::null_mut(),
            capacity: 0,
            length: 0,
        }
    }
}

impl<T> Clone for Vec<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let length = self.length;
        let capacity = self.length;
        let ptr = unsafe { libc::malloc(capacity * mem::size_of::<T>()) as *mut T };

        assert!(!ptr.is_null());

        unsafe { ptr::copy(ptr, self.ptr, length) };

        Self {
            ptr,
            capacity,
            length,
        }
    }
}

impl<T> Vec<T>
where
    T: Clone,
{
    pub fn at(&self, index: usize) -> Option<T> {
        match index < self.len() {
            true => unsafe { self.ptr.add(index).as_ref() }.cloned(),
            false => None,
        }
    }

    pub fn prepend(&mut self, data: &[T]) -> Result<(), libc::c_int>
    where
        T: Copy,
    {
        let new_length = self.length + data.len();

        while self.capacity < new_length {
            self.resize()?
        }

        unsafe { ptr::copy(self.ptr, self.ptr.add(data.len()), self.length) };

        for (i, &value) in data.iter().enumerate() {
            unsafe {
                ptr::write(self.ptr.add(i), value);
            }
        }

        self.length = new_length;

        Ok(())
    }
}

impl<T> Vec<T>
where
    T: PartialEq + Copy,
{
    pub fn find(&self, element: T) -> Option<usize> {
        for i in 0..self.len() {
            let e = unsafe { ptr::read_unaligned(self.ptr.add(i)) };
            if e == element {
                return Some(i);
            }
        }

        None
    }

    pub fn split(&self, element: T, empty_allowed: bool) -> Result<Vec<Vec<T>>, libc::c_int> {
        let mut result = Vec::<Vec<T>>::default();
        let mut temp = Vec::<T>::default();

        for i in 0..self.len() {
            let e = unsafe { ptr::read_unaligned(self.ptr.add(i)) };
            if e == element {
                if empty_allowed || !temp.empty() {
                    result.push(temp)?;
                    temp = Vec::<T>::default();
                }
            } else {
                temp.push(e)?
            }
        }

        if empty_allowed || !temp.empty() {
            result.push(temp)?;
        }

        Ok(result)
    }
}

pub struct IntoVecIter<T> {
    vec: Vec<T>,
    index: usize,
}

impl<T> Iterator for IntoVecIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.length {
            let value = unsafe {
                // Read the value at the current index
                ptr::read(self.vec.ptr.add(self.index))
            };
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
}

pub struct VecIter<'a, T> {
    vec: &'a Vec<T>,
    index: usize,
}

impl<'a, T> Iterator for VecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.length {
            let value = unsafe {
                // Get a reference to the value at the current index
                &*self.vec.ptr.add(self.index)
            };
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
}

impl<T> Vec<T> {
    pub fn iter(&'_ self) -> VecIter<'_, T> {
        VecIter {
            vec: self,
            index: 0,
        }
    }

    pub fn into_iter(self) -> IntoVecIter<T> {
        IntoVecIter {
            vec: self,
            index: 0,
        }
    }

    pub fn new(elements: &[T]) -> Result<Vec<T>, libc::c_int> {
        let length = elements.len();
        let ptr = unsafe { libc::malloc(core::mem::size_of_val(elements)) as *mut T };
        if ptr.is_null() {
            return Err(libc::ENOMEM);
        }

        unsafe { ptr::copy_nonoverlapping(elements.as_ptr(), ptr, length) };

        Ok(Vec {
            ptr,
            capacity: elements.len(),
            length,
        })
    }

    pub fn with_capacity(capacity: usize) -> Result<Vec<T>, libc::c_int> {
        let length = 0;
        let ptr = unsafe { libc::malloc(capacity * mem::size_of::<T>()) as *mut T };
        if ptr.is_null() {
            return Err(libc::ENOMEM);
        }

        Ok(Vec {
            ptr,
            capacity,
            length,
        })
    }

    pub fn empty(&self) -> bool {
        self.len() == 0
    }

    pub fn fill_by_function<F, E>(&mut self, fun: F) -> Result<(), E>
    where
        F: FnOnce(*mut T, usize) -> Result<usize, E>,
    {
        let read = fun(self.ptr, self.capacity)?;

        self.length = read;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn push(&mut self, value: T) -> Result<(), libc::c_int> {
        if self.length == self.capacity {
            self.resize()?;
        }
        unsafe { ptr::write_unaligned(self.ptr.add(self.length), value) };
        self.length += 1;

        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }

        let value = unsafe { ptr::read(self.ptr.add(self.length - 1)) };

        self.length -= 1;
        Some(value)
    }

    fn resize(&mut self) -> Result<(), libc::c_int> {
        let new_capacity = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };
        let new_ptr = unsafe { libc::malloc(new_capacity * mem::size_of::<T>()) as *mut T };

        if new_ptr.is_null() {
            return Err(libc::ENOMEM);
        }

        if !self.ptr.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(self.ptr, new_ptr, self.length);
                libc::free(self.ptr as *mut libc::c_void);
            }
        }

        self.ptr = new_ptr;
        self.capacity = new_capacity;

        Ok(())
    }

    pub fn as_slice(&self) -> Option<&[T]> {
        // Safety: We are returning a slice of the valid range of elements.
        // The pointer is valid for the length of the vector.
        match self.empty() {
            false => Some(unsafe { core::slice::from_raw_parts(self.ptr, self.length) }),
            true => None,
        }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                libc::free(self.ptr as *mut libc::c_void);
            }
        }
    }
}
