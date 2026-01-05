use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::io::{Read, Seek, SeekFrom};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use bitflags::bitflags;
use crate::from_slice;
use crate::schema::header::TableHeader;
use crate::schema::rows::{Row, RowValue};
use crate::utils::slice::FromSlice;
use crate::utils::endianness::BigEndian;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct ColumnFlag : u8 {
        const NAME = 1 << 4;
        const DEFAULT_VALUE = 1 << 5;
        const ROW_STORAGE = 1 << 6;
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnType {
    Byte = 0,
    SByte = 1,
    UInt16 = 2,
    Int16 = 3,
    UInt32 = 4,
    Int32 = 5,
    UInt64 = 6,
    Int64 = 7,
    Single = 8,
    Double = 9,
    String = 10,
    Data = 11,
    Guid = 12
}

impl ColumnType {
    pub(crate) fn get_size(&self) -> u32 {
        match self {
            Self::Byte | Self::SByte => 1,
            Self::UInt16 | Self::Int16 => 2,
            Self::UInt32 | Self::Int32 | Self::Single | Self::String => 4,
            Self::UInt64 | Self::Int64 | Self::Double | Self::Data => 8,
            Self::Guid => 16
        }
    }
}

const TYPE_MASK: u8 = 0xf;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColumnValue(u8);

impl ColumnValue {
    pub(crate) const fn get_flags(&self) -> ColumnFlag {
        ColumnFlag::from_bits_retain(self.0 & !TYPE_MASK)
    }
    pub(crate) const fn get_type(&self) -> ColumnType {
        unsafe { std::mem::transmute(self.0 & TYPE_MASK) }
    }
}

impl Debug for ColumnValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ColumnFlag {{ Type: {:?}, Flags: {:?} }}", self.get_type(), self.get_flags())
    }
}

#[derive(Debug)]
pub(crate) struct Column {
    flag: ColumnValue,
    string_offset: u32,
    default: Option<RowValue>
}

impl Column {
    pub(crate) fn new(flag: ColumnValue, string_offset: u32, default: Option<RowValue>) -> Self {
        Self { flag, string_offset, default }
    }

    pub(crate) fn get_value(&self) -> ColumnValue {
        self.flag
    }
    pub(crate) fn get_string_offset(&self) -> u32 {
        self.string_offset
    }

    pub(crate) fn get_default_value(&self) -> Option<&RowValue> {
        self.default.as_ref()
    }

    pub(crate) fn new_list<C: Read + Seek>(handle: &mut C, header: &TableHeader) -> Result<Vec<Self>, Box<dyn Error>> {
        handle.seek(SeekFrom::Start(crate::schema::header::HEADER_SIZE as u64))?;
        let mut columns: Vec<Self> = Vec::with_capacity(header.column_count() as usize);
        let mut current: MaybeUninit<[u8; 5]> = MaybeUninit::uninit(); // maximum possible size for row
        let mut default: MaybeUninit<[u8; 0x10]> = MaybeUninit::uninit();
        for _ in 0..header.column_count() as usize {
            handle.read_exact(unsafe { current.assume_init_mut() })?;
            let flag = ColumnValue(from_slice!(unsafe { current.assume_init_ref() }, u8));
            let string_offset = from_slice!(unsafe { current.assume_init_ref() }, u32, 0x1);
            let default = match flag.get_flags().contains(ColumnFlag::DEFAULT_VALUE) {
                true => {
                    let ctype = flag.get_type();
                    let slice = unsafe { std::slice::from_raw_parts_mut(
                        default.as_mut_ptr() as *mut u8, ctype.get_size() as usize) };
                    handle.read_exact(slice)?;
                    Some(Row::into_row_value(ctype, unsafe { default.assume_init_ref() }))
                },
                false => None
            };
            columns.push(Self::new(flag, string_offset, default));
        }
        Ok(columns)
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::io::{BufReader, Read, Seek, SeekFrom};
    use std::mem::MaybeUninit;
    use crate::schema::columns::{Column, ColumnFlag, ColumnType};
    use crate::schema::header::{TableHeader, HEADER_SIZE};
    use crate::schema::strings::{ StringPool, StringPoolImpl };

    #[test]
    fn read_columns_acb() -> Result<(), Box<dyn Error>> {
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
        let string_pool = StringPoolImpl::new(&mut handle, &header)?;

        let v0 = columns[0].get_value();
        assert_eq!(v0.get_type(), ColumnType::UInt32);
        assert_eq!(v0.get_flags(), ColumnFlag::NAME | ColumnFlag::ROW_STORAGE);
        assert_eq!(string_pool.get_string(columns[0].string_offset).unwrap(), "FileIdentifier");

        let v3 = columns[3].get_value();
        assert_eq!(v3.get_type(), ColumnType::Byte);
        assert_eq!(v3.get_flags(), ColumnFlag::NAME | ColumnFlag::ROW_STORAGE);
        assert_eq!(string_pool.get_string(columns[3].string_offset).unwrap(), "Type");

        let v5 = columns[5].get_value();
        assert_eq!(v5.get_type(), ColumnType::Data);
        assert_eq!(v5.get_flags(), ColumnFlag::NAME | ColumnFlag::ROW_STORAGE);
        assert_eq!(string_pool.get_string(columns[5].string_offset).unwrap(), "AcfMd5Hash");
        /*
        for c in &mut columns {
            println!("{:?} ({})", c, string_pool.get_string(c.string_offset).unwrap());
        }
        */
        Ok(())
    }
}