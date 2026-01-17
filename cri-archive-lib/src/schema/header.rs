//! # CRI Table Parser
//!
//! **Table Binary Format:**
//! - u32 Signature: 0x0,
//! - u32 Length: 0x4,
//! - hd_offset START: 0x8,
//! - ECriEncodingType EncodingType: 0x9,
//! - u16 RowsOffset: 0xa,
//! - u32 StringPoolOffset: 0xc,
//! - u32 DataPoolOffset: 0x10,
//! - TCriString Name: 0x14,
//! - u16 FieldCount: 0x18,
//! - u16 RowSize: 0x1a,
//! - u32 RowCount: 0x1c,

use std::fmt::{Debug, Formatter};
use std::ptr::NonNull;
use crate::from_slice;
use crate::utils::endianness::BigEndian;
use crate::utils::slice::FromSlice;

#[derive(Debug)]
#[repr(u8)]
pub enum StringEncoding {
    ShiftJIS,
    UTF8
}

impl From<u8> for StringEncoding {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::ShiftJIS,
            _ => Self::UTF8
        }
    }
}


pub(crate) static HEADER_OFFSET: u32 = 0x8;
pub static HEADER_SIZE: usize = 0x20;

//pub(crate) struct TableHeader<'a> {
pub struct TableHeader {
    /// Byte slice associated with this header instance. It's assumed that the slice is large
    /// enough to contain the entire table
    pub(crate) owner: NonNull<[u8]>,
}

impl TableHeader {
    pub fn new(file: &[u8]) -> Self {
        Self { owner: unsafe { NonNull::new_unchecked(&raw const* file as _) } }
    }

    pub fn size(&self) -> u32 {
        from_slice!(unsafe { self.owner.as_ref() }, u32, 0x4)
    }

    pub fn encoding(&self) -> StringEncoding {
        from_slice!(unsafe { self.owner.as_ref() }, u8, 0x9).into()
    }

    pub fn rows_offset(&self) -> u16 {
        from_slice!(unsafe { self.owner.as_ref() }, u16, 0xa) + HEADER_OFFSET as u16
    }

    pub fn string_pool_offset(&self) -> u32 {
        from_slice!(unsafe { self.owner.as_ref() }, u32, 0xc) + HEADER_OFFSET
    }

    pub fn data_pool_offset(&self) -> u32 {
        from_slice!(unsafe { self.owner.as_ref() }, u32, 0x10) + HEADER_OFFSET
    }

    // Field data
    pub fn column_count(&self) -> u16 {
        from_slice!(unsafe { self.owner.as_ref() }, u16, 0x18)
    }

    pub fn row_size(&self) -> u16 {
        from_slice!(unsafe { self.owner.as_ref() }, u16, 0x1a)
    }

    pub fn row_count(&self) -> u32 {
        from_slice!(unsafe { self.owner.as_ref() }, u32, 0x1c)
    }
}

impl Debug for TableHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TableHeader {{ size: 0x{:x}, encoding: {:?}, row_offset: 0x{:x}, string_pool_offset: 0x{:x}, data_pool_offset: 0x{:x}, column_count: 0x{:x}, row_size: 0x{:x}, row_count: 0x{:x} }}",
               self.size(), self.encoding(), self.rows_offset(), self.string_pool_offset(), self.data_pool_offset(), self.column_count(), self.row_size(), self.row_count())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::error::Error;
    use std::fs::File;
    use std::io::SeekFrom;
    use std::mem::MaybeUninit;
    use std::io::{Read, Seek};

    #[test]
    fn read_header_acb() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/bgm.acb";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let file = std::fs::read(target_table)?;
        let header = TableHeader::new(&file);
        assert_eq!(512, header.rows_offset());
        assert_eq!(951, header.string_pool_offset());
        assert_eq!(2112, header.data_pool_offset());
        assert_eq!(96, header.column_count());
        assert_eq!(415, header.row_size());
        assert_eq!(1, header.row_count());
        Ok(())
    }

    #[test]
    fn read_header_acf() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/sound.acf";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let file = std::fs::read(target_table)?;
        let header = TableHeader::new(&file);
        assert_eq!(352, header.rows_offset());
        assert_eq!(674, header.string_pool_offset());
        assert_eq!(1568, header.data_pool_offset());
        assert_eq!(64, header.column_count());
        assert_eq!(322, header.row_size());
        assert_eq!(1, header.row_count());
        Ok(())
    }

    #[test]
    fn read_header_cpk() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/SteamLibrary/steamapps/common/METAPHOR/base.cpk";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = File::open(target_table)?;
        handle.seek(SeekFrom::Start(0x10))?; // go to first table
        // this CPK table is not encrypted so we can immediately use TableHeader
        let mut first_header: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read_exact(unsafe { first_header.assume_init_mut() })?;
        let first_header = unsafe { first_header.assume_init() };
        let header = TableHeader::new(&first_header);
        assert_eq!(252, header.rows_offset());
        assert_eq!(378, header.string_pool_offset());
        assert_eq!(848, header.data_pool_offset());
        assert_eq!(44, header.column_count());
        assert_eq!(126, header.row_size());
        assert_eq!(1, header.row_count());
        Ok(())
    }
}