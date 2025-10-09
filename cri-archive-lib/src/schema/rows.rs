use std::error::Error;
use std::io::{Read, Seek, SeekFrom};
use std::mem::MaybeUninit;
use std::ops::Index;
use crate::schema::columns::{Column, ColumnType};
use crate::schema::header::TableHeader;
use crate::utils::endianness::BigEndian;
use crate::utils::slice::FromSlice;
use crate::from_slice;

#[derive(Debug, PartialEq)]
pub(crate) enum RowValue {
    Byte(u8),
    SByte(i8),
    UInt16(u16),
    Int16(i16),
    UInt32(u32),
    Int32(i32),
    UInt64(u64),
    Int64(i64),
    Single(f32),
    Double(f64),
    String(u32),
    Data(DataValue),
    Guid([u32; 4])
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DataValue {
    offset: u32,
    length: u32
}

impl DataValue {
    pub(crate) fn is_none(&self) -> bool {
        self.length == 0
    }
}

#[derive(Debug)]
pub(crate) struct RowOffsets<'a> {
    columns: &'a [Column],
    rows: Vec<RowValue>
}

impl<'a> RowOffsets<'a> {
    pub(crate) fn new<C: Read + Seek>(handle: &mut C, header: &TableHeader,
        columns: &'a [Column]) -> Result<Self, Box<dyn Error>> {
        handle.seek(SeekFrom::Start(header.rows_offset as u64))?;
        let mut rows = vec![];
        let mut field: MaybeUninit<[u8; 0x10]> = MaybeUninit::uninit();
        for c in columns {
            let ctype = c.get_value().get_type();
            let slice = unsafe { std::slice::from_raw_parts_mut(
                field.as_mut_ptr() as *mut u8, ctype.get_size() as usize) };
            handle.read(slice)?;

            rows.push(match c.get_value().get_type() {
                ColumnType::Byte => RowValue::Byte(unsafe { field.assume_init_ref()[0] }),
                ColumnType::SByte => RowValue::SByte(unsafe { field.assume_init_ref()[0] as i8 }),
                ColumnType::UInt16 => RowValue::UInt16(from_slice!(unsafe { &field.assume_init_ref()[0..2] }, u16)),
                ColumnType::Int16 => RowValue::Int16(from_slice!(unsafe { &field.assume_init_ref()[0..2] }, i16)),
                ColumnType::UInt32 => RowValue::UInt32(from_slice!(unsafe { &field.assume_init_ref()[0..4] }, u32)),
                ColumnType::Int32 => RowValue::Int32(from_slice!(unsafe { &field.assume_init_ref()[0..4] }, i32)),
                ColumnType::Single => RowValue::Single(from_slice!(unsafe { &field.assume_init_ref()[0..4] }, f32)),
                ColumnType::String => RowValue::String(from_slice!(unsafe { &field.assume_init_ref()[0..4] }, u32)),
                ColumnType::UInt64 => RowValue::UInt64(from_slice!(unsafe { &field.assume_init_ref()[0..8] }, u64)),
                ColumnType::Int64 => RowValue::Int64(from_slice!(unsafe { &field.assume_init_ref()[0..8] }, i64)),
                ColumnType::Double => RowValue::Double(from_slice!(unsafe { &field.assume_init_ref()[0..8] }, f64)),
                ColumnType::Data => RowValue::Data(DataValue {
                    offset: from_slice!(unsafe { &field.assume_init_ref()[0..4] }, u32),
                    length: from_slice!(unsafe { &field.assume_init_ref()[4..8] }, u32),
                }),
                ColumnType::Guid => todo!()
            });
            // handle.seek(SeekFrom::Current(c.get_value().get_type().get_size() as i64))?;
        }
        Ok(Self { columns, rows })
    }
}

impl<'a> Index<usize> for RowOffsets<'a> {
    type Output = RowValue;
    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[index]
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::mem::MaybeUninit;
    use crate::schema::columns::Column;
    use crate::schema::header::{TableHeader, HEADER_SIZE};
    use crate::schema::rows::{DataValue, RowOffsets, RowValue};
    use crate::schema::strings::StringPool;

    #[test]
    fn read_rows_acb() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/Metaphor/base_cpk/COMMON/sound/bgm.acb";
        if !std::fs::exists(target_table)? {
            return Ok(());
        }
        let mut handle = BufReader::new(File::open(target_table)?);
        let mut header_serial: MaybeUninit<[u8; HEADER_SIZE]> = MaybeUninit::uninit();
        handle.read(unsafe { header_serial.assume_init_mut() })?;
        let header_serial = unsafe { header_serial.assume_init() };
        let header = TableHeader::new(&header_serial)?;
        let columns = Column::new_list(&mut handle, &header)?;
        let string_pool = StringPool::new(&mut handle, &header)?;
        let rows =  RowOffsets::new(&mut handle, &header, &columns)?;
        assert_eq!(RowValue::UInt32(0), rows[0]);
        assert_eq!(RowValue::UInt32(20382208), rows[2]);
        assert_eq!(RowValue::Data(DataValue { offset: 0, length: 16 }), rows[5]);
        assert_eq!(RowValue::Data(DataValue { offset: 32, length: 1704 }), rows[7]);
        Ok(())
    }
}