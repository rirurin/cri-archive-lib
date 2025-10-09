use std::collections::HashMap;
use std::error::Error;
use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom};
use std::ptr::NonNull;
use crate::schema::header::TableHeader;

#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct StringPool(Vec<u8>);

impl StringPool {
    pub(crate) fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        handle.seek(SeekFrom::Start(header.string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset - header.string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read(&mut alloc)?;
        Ok(Self(alloc))
    }

    pub(crate) fn get_string(&self, offset: u32) -> Option<&str> {
        if (offset as usize) < self.0.len() {
            Some(unsafe { CStr::from_ptr(self.0.as_ptr().add(offset as usize) as _).to_str().unwrap() })
        } else {
            None
        }
    }
}

pub(crate) struct StringPoolFast {
    storage: Vec<u8>,
    pointers: HashMap<usize, NonNull<CStr>>
}

impl StringPoolFast {
    pub(crate) fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        handle.seek(SeekFrom::Start(header.string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset - header.string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read(&mut alloc)?;
        let mut offset = 0;
        let mut pointers = HashMap::new();
        while offset < pool_length {
            // CStr::from_ptr precalculates strlen, so cache the result for each string
            // Calls to StringPoolFast::get_string() will use O(1) CStr::to_str()
            let new = unsafe { CStr::from_ptr(alloc.as_ptr().add(offset) as _) };
            pointers.insert(offset, unsafe { NonNull::new_unchecked(&raw const *new as *mut CStr) });
            offset += new.count_bytes() + 1;
        }
        Ok(Self {
            storage: alloc,
            pointers
        })
    }

    pub(crate) fn get_string(&self, offset: u32) -> Option<&str> {
        self.pointers.get(&(offset as usize)).map(|s| unsafe { s.as_ref().to_str().unwrap() })
    }
}