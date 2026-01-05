use std::collections::HashMap;
use std::error::Error;
use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom};
use std::ptr::NonNull;
use crate::schema::header::TableHeader;

pub(crate) trait StringPool {
    fn get_string(&self, offset: u32) -> Option<&str>;
}

#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct StringPoolImpl(Vec<u8>);

impl StringPoolImpl {
    pub(crate) fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        let string_pool_offset = header.string_pool_offset();
        handle.seek(SeekFrom::Start(string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset() - string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read_exact(&mut alloc)?;
        Ok(Self(alloc))
    }
}

impl StringPool for StringPoolImpl {
    fn get_string(&self, offset: u32) -> Option<&str> {
        if (offset as usize) < self.0.len() {
            Some(unsafe { CStr::from_ptr(self.0.as_ptr().add(offset as usize) as _).to_str().unwrap() })
        } else {
            None
        }
    }
}

// Faster than StringPoolImpl in Release mode
// This assumes an outer structure which is holding a valid reference to the byte stream
// containing these strings (e.g HighTable in cpk::reader) to avoid making another copy
#[derive(Debug)]
pub(crate) struct StringPoolFast(HashMap<usize, NonNull<str>>);

impl StringPoolFast {
    pub(crate) fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        let string_pool_offset = header.string_pool_offset();
        handle.seek(SeekFrom::Start(string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset() - string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read_exact(&mut alloc)?;
        unsafe { Self::new_borrowed(&alloc) }
    }

    // Assumes that this slice begins at string_pool_offset
    pub(crate) unsafe fn new_borrowed(stream: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut offset = 0;
        let mut pointers = HashMap::new();
        while offset < stream.len() {
            // CStr::from_ptr precalculates strlen, so cache the result for each string
            // Calls to StringPoolFast::get_string() will use O(1) CStr::to_str()
            let new = unsafe { CStr::from_ptr(stream.as_ptr().add(offset) as _).to_str().unwrap() };
            pointers.insert(offset, unsafe { NonNull::new_unchecked(&raw const *new as *mut str) });
            offset += new.len() + 1;
        }
        Ok(Self(pointers))
    }
}

impl StringPool for StringPoolFast {
    fn get_string(&self, offset: u32) -> Option<&str> {
        self.0.get(&(offset as usize)).map(|s| unsafe { s.as_ref() })
    }
}

#[cfg(test)]
pub mod tests {
    /*
    use std::error::Error;
    use std::fs::File;
    use std::hint::black_box;
    use std::io::{BufReader, Read};
    use std::mem::MaybeUninit;
    use std::time::Instant;
    use crate::schema::columns::{Column, ColumnFlag};
    use crate::schema::header::{TableHeader, HEADER_SIZE};
    use crate::schema::strings::{StringPool, StringPoolFast, StringPoolImpl};

    #[test]
    fn read_strings_from_acb() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/bgm.acb";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = BufReader::new(File::open(target_table)?);
        let mut header_serial: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read_exact(unsafe { header_serial.assume_init_mut() })?;
        let header_serial = unsafe { header_serial.assume_init() };
        let header = TableHeader::new(&header_serial);
        let columns = Column::new_list(&mut handle, &header)?;
        let string_pool = StringPoolFast::new(&mut handle, &header)?;
        let start = Instant::now();
        for _ in 0..1000 {
            for col in &columns {
                if col.get_value().get_flags().contains(ColumnFlag::NAME) {
                    black_box(string_pool.get_string(col.get_offset()));
                }
            }
        }
        println!("completed in {} us", Instant::now().duration_since(start).as_micros() as f64);
        Ok(())
    }
    */
}