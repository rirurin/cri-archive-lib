use std::collections::HashMap;
use std::error::Error;
use std::ffi::CStr;
use std::io::{Read, Seek, SeekFrom};
// use std::ptr::NonNull;
use encoding_rs::SHIFT_JIS;
use crate::schema::header::{StringEncoding, TableHeader};

pub trait StringPool {
    fn get_string(&self, offset: u32) -> Option<&str>;
}

#[derive(Debug)]
pub struct StringPoolImpl {
    alloc: Vec<u8>,
    encoding: StringEncoding
}

impl StringPoolImpl {
    pub fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        let string_pool_offset = header.string_pool_offset();
        handle.seek(SeekFrom::Start(string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset() - string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read_exact(&mut alloc)?;
        Ok(Self { alloc, encoding: header.encoding() })
    }
}

impl StringPool for StringPoolImpl {
    fn get_string(&self, offset: u32) -> Option<&str> {
        if (offset as usize) >= self.alloc.len() {
            return None;
        }
        match self.encoding {
            StringEncoding::ShiftJIS => panic!("StringPoolImpl does not support SHIFT-JIS"),
            StringEncoding::UTF8 => Some(unsafe { CStr::from_ptr(
                self.alloc.as_ptr().add(offset as usize) as _).to_str().unwrap() })
        }
    }
}

// Faster than StringPoolImpl in Release mode
// This assumes an outer structure which is holding a valid reference to the byte stream
// containing these strings (e.g HighTable in cpk::reader) to avoid making another copy
#[derive(Debug)]
// pub struct StringPoolFast(HashMap<usize, NonNull<str>>);
pub struct StringPoolFast(HashMap<usize, String>);

impl StringPoolFast {
    pub fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader)
        -> Result<Self, Box<dyn Error>> {
        let string_pool_offset = header.string_pool_offset();
        handle.seek(SeekFrom::Start(string_pool_offset as u64))?;
        let pool_length = (header.data_pool_offset() - string_pool_offset) as usize;
        let mut alloc = Vec::with_capacity(pool_length);
        unsafe { alloc.set_len(pool_length) };
        handle.read_exact(&mut alloc)?;
        unsafe { Self::new_borrowed(&alloc, header) }
    }

    // Assumes that this slice begins at string_pool_offset
    pub(crate) unsafe fn new_borrowed(stream: &[u8], header: &TableHeader) -> Result<Self, Box<dyn Error>> {
        match header.encoding() {
            StringEncoding::ShiftJIS => Self::new_borrowed_shift_jis(stream),
            StringEncoding::UTF8 => Self::new_borrowed_utf8(stream),
        }
    }

    fn new_borrowed_shift_jis(stream: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut offset = 0;
        let mut pointers = HashMap::new();
        while offset < stream.len() {
            let bytes = unsafe { CStr::from_ptr(stream.as_ptr().add(offset) as _).to_bytes() };
            let (new, _, _) = SHIFT_JIS.decode(bytes);
            pointers.insert(offset, new.to_string());
            offset += bytes.len() + 1;
        }
        Ok(Self(pointers))
    }

    fn new_borrowed_utf8(stream: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut offset = 0;
        let mut pointers = HashMap::new();
        while offset < stream.len() {
            // CStr::from_ptr precalculates strlen, so cache the result for each string
            // Calls to StringPoolFast::get_string() will use O(1) CStr::to_str()
            let new = unsafe { CStr::from_ptr(stream.as_ptr().add(offset) as _).to_str().unwrap() };
            pointers.insert(offset, new.to_string());
            offset += new.len() + 1;
        }
        Ok(Self(pointers))
    }
}

impl StringPool for StringPoolFast {
    fn get_string(&self, offset: u32) -> Option<&str> {
        self.0.get(&(offset as usize)).map(|s| s.as_ref())
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::io::{BufReader, Read/* , Seek, SeekFrom*/};
    use std::mem::MaybeUninit;
    use crate::schema::header::{TableHeader, HEADER_SIZE};
    use crate::schema::strings::{StringPoolFast, StringPoolImpl};

    #[test]
    fn parse_strings_fastpool_utf8() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/bgm.acb";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = BufReader::new(File::open(target_table)?);
        let mut header_serial: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read_exact(unsafe { header_serial.assume_init_mut() })?;
        let header_serial = unsafe { header_serial.assume_init() };
        let header = TableHeader::new(&header_serial);
        let _ = StringPoolFast::new(&mut handle, &header)?;
        Ok(())
    }

    #[test]
    fn parse_strings_standard_utf8() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/bgm.acb";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = BufReader::new(File::open(target_table)?);
        let mut header_serial: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read_exact(unsafe { header_serial.assume_init_mut() })?;
        let header_serial = unsafe { header_serial.assume_init() };
        let header = TableHeader::new(&header_serial);
        let _ = StringPoolImpl::new(&mut handle, &header)?;
        Ok(())
    }

    /*
    #[test]
    fn parse_strings_fastpool_shiftjis() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/SteamLibrary/steamapps/common/Persona 4 Golden/data.cpk";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = BufReader::new(File::open(target_table)?);
        handle.seek(SeekFrom::Start(0x810))?;
        let mut header_serial: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read_exact(unsafe { header_serial.assume_init_mut() })?;
        let header_serial = unsafe { header_serial.assume_init() };
        let header = TableHeader::new(&header_serial);
        let mut table = Vec::with_capacity(header.size() as usize + 8);
        unsafe { table.set_len(table.capacity()) };
        handle.read_exact(&mut table)?;
        let str_raw = &table[header.string_pool_offset() as usize..header.data_pool_offset() as usize];
        let _ = unsafe { StringPoolFast::new_borrowed(str_raw, &header)? };
        // let _ = unsafe { StringPoolFast::new(&mut handle, &header)? };
        /*
        let mut table = Vec::with_capacity(header.size() as usize);
        unsafe { table.set_len(table.capacity()) };
        handle.read_exact(&mut table)?;
        let str_raw = &table[header.string_pool_offset() as usize..header.data_pool_offset() as usize];
        let _ = unsafe { StringPoolFast::new_borrowed(str_raw, &header)? };
        */
        Ok(())
    }
    */
}