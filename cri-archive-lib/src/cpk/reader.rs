#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use std::mem::MaybeUninit;
    use crate::schema::columns::Column;
    use crate::schema::header::{TableHeader, HEADER_SIZE};
    use crate::schema::rows::RowOffsets;
    use crate::schema::strings::StringPool;

    #[test]
    fn get_files() -> Result<(), Box<dyn Error>> {
        let target_table = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData.cpk";
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
        Ok(())
    }
}