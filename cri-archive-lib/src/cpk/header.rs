use std::error::Error;
use std::io::{Cursor, Read, Seek};
use std::mem::MaybeUninit;
use crate::cpk::encrypt::table::TableDecryptor;
use crate::from_slice;
use crate::schema::columns::Column;
use crate::schema::header::TableHeader;
use crate::schema::rows::Row;
use crate::schema::strings::{StringPool, StringPoolFast};
use crate::utils::slice::FromSlice;
use crate::utils::endianness::NativeEndian;

#[derive(Debug)]
pub struct TableContainer;

impl TableContainer {
    pub fn new<R: Read + Seek>(stream: &mut R) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut table_header: MaybeUninit<[u8; 0x10]> = MaybeUninit::uninit();
        stream.read_exact(unsafe { table_header.assume_init_mut() })?;
        let table_header = unsafe { table_header.assume_init() };
        let size = from_slice!(&table_header, u32, NativeEndian, 0x8) as usize;
        let mut table = Vec::with_capacity(size);
        unsafe { table.set_len(table.capacity()) };
        stream.read_exact(&mut table)?;
        if TableDecryptor::is_encrypted(&table) {
            TableDecryptor::decrypt_utf_in_place(&mut table);
        }
        Ok(table)
    }
}

#[derive(Debug)]
pub(crate) struct HighTable<S: StringPool> {
    #[allow(dead_code)]
    alloc: Vec<u8>,
    #[allow(dead_code)]
    header: TableHeader,
    columns: Vec<Column>,
    strings: S,
    rows: Vec<Row>
}

impl HighTable<StringPoolFast> {
    pub fn new(alloc: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let header = TableHeader::new(&alloc);
        let mut cursor = Cursor::new(alloc.as_slice());
        cursor.set_position(crate::schema::header::HEADER_SIZE as u64);
        let columns = Column::new_list(&mut cursor, &header)?;
        let str_raw = &alloc[header.string_pool_offset() as usize..header.data_pool_offset() as usize];
        let rows = Row::new_list(&mut cursor, &header, columns.as_ref())?;
        let strings = unsafe { StringPoolFast::new_borrowed(&str_raw, &header)? };
        Ok(Self { alloc, header, columns, strings, rows })
    }
}

impl<S: StringPool> HighTable<S> {
    pub fn get_columns(&self) -> &[Column] { self.columns.as_ref() }
    pub fn get_strings(&self) -> &S { &self.strings }
    pub fn get_rows(&self) -> &[Row] { &self.rows }
    #[allow(dead_code)]
    pub fn get_alloc(&self) -> &[u8] { &self.alloc }
}