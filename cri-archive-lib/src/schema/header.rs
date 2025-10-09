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

use std::error::Error;
use std::fmt::{Debug, Formatter};
use crate::from_slice;
use crate::utils::endianness::BigEndian;
use crate::utils::slice::FromSlice;

pub(crate) struct TableHeader<'a> {
    /// Byte stream associated with this header instance
    pub(crate) owner: &'a [u8],
    /// Offset of rows relative to start of table
    pub(crate) rows_offset: u16,
    /// Offset of encoded strings relative to start of table
    pub(crate) string_pool_offset: u32,
    /// Offset of raw data relative to start of table
    pub(crate) data_pool_offset: u32,

    /// Number of columns in this table
    pub(crate) column_count: u16,
    /// Size of each row in bytes
    pub(crate) row_size: u16,
    /// Number of rows in this table
    pub(crate) row_count: u32
}

impl<'a> Debug for TableHeader<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TableHeader {{ owner: [ 0x{:x}, {} bytes ], rows_offset: {}, string_pool_offset: {}, \
data_pool_offset: {}, column_count: {}, row_size: {}, row_count: {} }}",
               self.owner.as_ptr() as usize, self.owner.len(), self.rows_offset, self.string_pool_offset,
        self.data_pool_offset, self.column_count, self.row_size, self.row_count)
    }
}

#[derive(Debug)]
pub(crate) enum StringEncoding {
    ShiftJIS,
    UTF8
}


pub(crate) static HEADER_OFFSET: u32 = 0x8;
pub(crate) static HEADER_SIZE: usize = 0x20;

impl<'a> TableHeader<'a> {
    pub(crate) fn new(file: &'a [u8]) -> Result<Self, Box<dyn Error>> {
        // Get offsets from header beginning (stream + 0x8)
        let rows_offset = from_slice!(file, u16, 0xa) + HEADER_OFFSET as u16;
        let string_pool_offset = from_slice!(file, u32, 0xc) + HEADER_OFFSET;
        let data_pool_offset = from_slice!(file, u32, 0x10) + HEADER_OFFSET;

        // Field data
        let column_count = from_slice!(file, u16, 0x18);
        let row_size = from_slice!(file, u16, 0x1a);
        let row_count = from_slice!(file, u32, 0x1c);

        Ok(Self {
            owner: file,
            rows_offset,
            string_pool_offset,
            data_pool_offset,
            column_count,
            row_size,
            row_count
        })
    }

    pub(crate) fn get_encoding(&self) -> StringEncoding {
        match self.owner[9] == 0 {
            true => StringEncoding::ShiftJIS,
            false => StringEncoding::UTF8
        }
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
        let header = TableHeader::new(&file)?;
        assert_eq!(512, header.rows_offset);
        assert_eq!(951, header.string_pool_offset);
        assert_eq!(2112, header.data_pool_offset);
        assert_eq!(96, header.column_count);
        assert_eq!(415, header.row_size);
        assert_eq!(1, header.row_count);
        Ok(())
    }

    #[test]
    fn read_header_acf() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/sound.acf";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let file = std::fs::read(target_table)?;
        let header = TableHeader::new(&file)?;
        assert_eq!(352, header.rows_offset);
        assert_eq!(674, header.string_pool_offset);
        assert_eq!(1568, header.data_pool_offset);
        assert_eq!(64, header.column_count);
        assert_eq!(322, header.row_size);
        assert_eq!(1, header.row_count);
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
        let mut first_header: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read(unsafe { first_header.assume_init_mut() })?;
        let first_header = unsafe { first_header.assume_init() };
        let header = TableHeader::new(&first_header)?;
        assert_eq!(252, header.rows_offset);
        assert_eq!(378, header.string_pool_offset);
        assert_eq!(848, header.data_pool_offset);
        assert_eq!(44, header.column_count);
        assert_eq!(126, header.row_size);
        assert_eq!(1, header.row_count);
        Ok(())
    }

}