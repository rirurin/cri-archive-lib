use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use crate::cpk::compress::layla::LaylaDecompressor;
use crate::cpk::encrypt::p5r::P5RDecryptor;
use crate::cpk::file::CpkFile;
use crate::cpk::header::{HighTable, TableContainer};
use crate::schema::columns::{Column, ColumnFlag, ColumnType};
use crate::schema::rows::{Row, RowValue};
use crate::schema::strings::{ StringPool, StringPoolFast };

#[derive(Debug)]
pub enum CpkReaderError {
    MissingTocOffset,
    MissingContentOffset,
    NoFileName,
    NoFileSize,
    NoExtractSize,
    GetFilesNotCalled,
}

impl Error for CpkReaderError {}

impl Display for CpkReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

#[derive(Debug)]
pub struct CpkReader<R: Read + Seek> {
    stream: R,
    start_pos: u64,
    content_ofs: u64,
    toc_table: Option<HighTable<StringPoolFast>>,
    decryption: Option<P5RDecryptor>
}

impl<R> CpkReader<R> where R: Read + Seek {
    const DEFAULT_OFFSET: u64 = u64::MAX;

    pub fn new(stream: R) -> Result<Self, Box<dyn Error>> {
        Self::new_inner(stream, None)
    }

    pub fn new_p5r(stream: R) -> Result<Self, Box<dyn Error>> {
        Self::new_inner(stream, Some(P5RDecryptor))
    }

    fn new_inner(mut stream: R, decryption: Option<P5RDecryptor>) -> Result<Self, Box<dyn Error>> {
        let start_pos = stream.stream_position()?;
        Ok(Self { stream, start_pos, content_ofs: Self::DEFAULT_OFFSET, toc_table: None, decryption })
    }

    pub fn get_files(&mut self) -> Result<Vec<CpkFile>, Box<dyn Error>> {
        self.stream.seek(SeekFrom::Start(self.start_pos))?;
        // Read CPK table to get offset to TOC and Content
        let cpk_table = HighTable::<StringPoolFast>::new(
            TableContainer::new(&mut self.stream)?)?;
        let cpk_strs = cpk_table.get_strings();
        let mut toc_offset= Self::DEFAULT_OFFSET;
        for (col, row) in cpk_table.get_columns().iter()
            .zip(cpk_table.get_rows()[0].iter()) {
            if toc_offset != Self::DEFAULT_OFFSET && self.content_ofs != Self::DEFAULT_OFFSET { break; }
            if col.get_value().get_flags().contains(ColumnFlag::NAME | ColumnFlag::ROW_STORAGE) {
                if let RowValue::UInt64(v) = row {
                    if let Some(str) = cpk_strs.get_string(col.get_string_offset()) {
                        match str {
                            "TocOffset" => toc_offset = *v,
                            // cache content offset for extract_file calls
                            "ContentOffset" => self.content_ofs = *v,
                            _ => ()
                        }
                    }
                }
            }
        }
        if toc_offset == Self::DEFAULT_OFFSET { return Err(Box::new(CpkReaderError::MissingTocOffset)) }
        if self.content_ofs == Self::DEFAULT_OFFSET { return Err(Box::new(CpkReaderError::MissingContentOffset)) }
        // In some CPKs offsets are relative to TOC as opposed to ContentOffset in header.
        // This happens when TOC address is before ContentOffset.
        if toc_offset < self.content_ofs {
            self.content_ofs = toc_offset;
        }
        // Read and cache TOC table
        self.stream.seek(SeekFrom::Start(toc_offset))?;
        self.toc_table = Some(HighTable::<StringPoolFast>::new(
            TableContainer::new(&mut self.stream)?)?);
        let toc_table = self.toc_table.as_mut().unwrap();
        let toc_str = toc_table.get_strings();
        let toc_indices = TocTableIndices::new(toc_str, toc_table.get_columns());
        let toc_col = toc_table.get_columns();
        let files = toc_table.get_rows();
        let mut out = Vec::with_capacity(files.len());
        for file in files {
            let directory_name = file.cpk_get_directory_name(
                &toc_col[toc_indices.dir_name], toc_str, toc_indices.dir_name)?;
            let file_name = file.cpk_get_file_name(toc_str, toc_indices.file_name)?;
            let file_offset = file.cpk_get_file_offset(toc_indices.file_offset)?;
            let file_size = file.cpk_get_file_size(toc_indices.file_size)?;
            let extract_size = file.cpk_get_extract_size(toc_indices.extract_size)?;
            let user_string = file.cpk_get_user_string(
                &toc_col[toc_indices.user_string], toc_str, toc_indices.user_string)?;
            out.push(CpkFile::new(directory_name, file_name, file_offset, file_size, extract_size, user_string))
        }
        Ok(out)
    }

    pub fn extract_file(&mut self, file: &CpkFile) -> Result<Vec<u8>, Box<dyn Error>> {
        // println!("{}/{}, size: 0x{:x}/0x{:x}, ofs: 0x{:x}, user: {}", file.directory(), file.file_name(), file.file_size(), file.extract_size(), file.file_offset(), file.user_string());
        if self.content_ofs == Self::DEFAULT_OFFSET { return Err(Box::new(CpkReaderError::GetFilesNotCalled)) }
        self.stream.seek(SeekFrom::Start(self.content_ofs + file.file_offset()))?;
        let mut out = Vec::with_capacity(file.file_size() as usize);
        unsafe { out.set_len(out.capacity()) };
        self.stream.read_exact(&mut out)?;
        if self.decryption.is_some() {
            P5RDecryptor::decrypt_in_place(&mut out);
        }
        Ok(match LaylaDecompressor::is_compressed(&out) {
            true => LaylaDecompressor::decompress(&out),
            false => out
        })
    }
}

impl Row {
    pub(crate) fn cpk_get_file_name<'a, S: StringPool>(&'a self, string_pool: &'a S, col_index: usize)
        -> Result<&'a str, Box<dyn Error>> {
        match self[col_index] {
            RowValue::String(ofs) => string_pool.get_string(ofs).map_or(
                Err(Box::new(CpkReaderError::NoFileName)), |v| Ok(v)),
            _ => Err(Box::new(CpkReaderError::NoFileName))
        }
    }

    fn cpk_get_u32_value(&self, col_index: usize) -> Result<u32, Box<dyn Error>> {
        match self[col_index] {
            RowValue::UInt32(size) => Ok(size),
            _ => Err(Box::new(CpkReaderError::NoFileName))
        }
    }

    pub(crate) fn cpk_get_file_size(&self, col_index: usize)
        -> Result<u32, Box<dyn Error>> {
        self.cpk_get_u32_value(col_index)
    }

    pub(crate) fn cpk_get_extract_size(&self, col_index: usize)
        -> Result<u32, Box<dyn Error>> {
        self.cpk_get_u32_value(col_index)
    }

    pub(crate) fn cpk_get_file_offset(&self, col_index: usize) -> Result<u64, Box<dyn Error>> {
        match self[col_index] {
            RowValue::UInt64(size) => Ok(size),
            _ => Err(Box::new(CpkReaderError::NoFileName))
        }
    }

    pub(crate) fn cpk_get_string_may_default<'a, S: StringPool>(&'a self, column: &Column,
        string_pool: &'a S, col_index: usize) -> Result<&'a str, Box<dyn Error>> {
        if let RowValue::String(ofs) = self[col_index] {
            if let Some(str) = string_pool.get_string(ofs) {
                return Ok(str);
            }
        } else if let RowValue::None = self[col_index] {
            if let Some(def) = column.get_default_value() {
                if let RowValue::String(ofs) = *def {
                    if let Some(str) = string_pool.get_string(ofs) {
                        return Ok(str);
                    }
                }
            }
        }
        Err(Box::new(CpkReaderError::NoFileName))
    }

    pub(crate) fn cpk_get_directory_name<'a, S: StringPool>(&'a self, column: &Column,
        string_pool: &'a S, col_index: usize) -> Result<&'a str, Box<dyn Error>> {
        self.cpk_get_string_may_default(column, string_pool, col_index)
    }

    pub(crate) fn cpk_get_user_string<'a, S: StringPool>(&'a self, column: &Column,
        string_pool: &'a S, col_index: usize) -> Result<&'a str, Box<dyn Error>> {
        self.cpk_get_string_may_default(column, string_pool, col_index)
    }
}

#[derive(Debug)]
struct TocTableIndices {
    dir_name: usize,
    file_name: usize,
    file_size: usize,
    extract_size: usize,
    file_offset: usize,
    user_string: usize
}

impl TocTableIndices {
    pub(crate) fn new<S: StringPool>(pool: &S, cols: &[Column]) -> Self {
        let mut inst = Self {
            dir_name: usize::MAX,
            file_name: usize::MAX,
            file_size: usize::MAX,
            extract_size: usize::MAX,
            file_offset: usize::MAX,
            user_string: usize::MAX
        };
        for (i, c) in cols.iter().enumerate() {
            if let Some(s) = pool.get_string(c.get_string_offset()) {
                match s {
                    "DirName" => inst.dir_name = i,
                    "FileName" => inst.file_name = i,
                    "FileSize" => inst.file_size = i,
                    "ExtractSize" => inst.extract_size = i,
                    "FileOffset" => inst.file_offset = i,
                    "UserString" => inst.user_string = i,
                    _ => ()
                }
            }
        }
        inst
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs::File;
    use std::io::BufReader;
    use crate::cpk::compress::layla::LaylaDecompressor;
    use crate::cpk::reader::CpkReader;

    #[test]
    fn get_files_basic_table() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData.cpk";
        if !std::fs::exists(sample_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new(BufReader::new(File::open(sample_path)?))?;
        let files = reader.get_files()?;
        assert_eq!(files[0].directory(), "");
        assert_eq!(files[0].file_name(), "Audio-NoCompression.flac");
        assert_eq!(files[0].file_offset(), 0);
        assert_eq!(files[0].file_size(), 48431);
        assert_eq!(files[0].extract_size(), 48431);
        assert_eq!(files[0].user_string(), "<NULL>");
        assert_eq!(files[1].directory(), "");
        assert_eq!(files[1].file_name(), "Image-NoCompression.jpg");
        assert_eq!(files[1].file_offset(), 48640);
        assert_eq!(files[1].file_size(), 120719);
        assert_eq!(files[1].extract_size(), 120719);
        assert_eq!(files[1].user_string(), "<NULL>");
        assert_eq!(files[2].directory(), "");
        assert_eq!(files[2].file_name(), "Text-Compressed.txt");
        assert_eq!(files[2].file_offset(), 169472);
        assert_eq!(files[2].file_size(), 2484);
        assert_eq!(files[2].extract_size(), 3592);
        assert_eq!(files[2].user_string(), "<NULL>");
        Ok(())
    }

    #[test]
    fn get_files_encrypted_table() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData-Encrypted.cpk";
        if !std::fs::exists(sample_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new(BufReader::new(File::open(sample_path)?))?;
        let files = reader.get_files()?;
        assert_eq!(files[0].directory(), "");
        assert_eq!(files[0].file_name(), "Audio-NoCompression.flac");
        assert_eq!(files[0].file_offset(), 0);
        assert_eq!(files[0].file_size(), 48431);
        assert_eq!(files[0].extract_size(), 48431);
        assert_eq!(files[0].user_string(), "<NULL>");
        assert_eq!(files[1].directory(), "");
        assert_eq!(files[1].file_name(), "Image-NoCompression.jpg");
        assert_eq!(files[1].file_offset(), 48640);
        assert_eq!(files[1].file_size(), 120719);
        assert_eq!(files[1].extract_size(), 120719);
        assert_eq!(files[1].user_string(), "<NULL>");
        assert_eq!(files[2].directory(), "");
        assert_eq!(files[2].file_name(), "Text-Compressed.txt");
        assert_eq!(files[2].file_offset(), 169472);
        assert_eq!(files[2].file_size(), 2484);
        assert_eq!(files[2].extract_size(), 3592);
        assert_eq!(files[2].user_string(), "<NULL>");
        Ok(())
    }

    #[test]
    fn extract_sample_image_uncompressed() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData-Encrypted.cpk";
        let expected_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData/Image-NoCompression.jpg";
        if !std::fs::exists(sample_path)? || !std::fs::exists(expected_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new(BufReader::new(File::open(sample_path)?))?;
        let files = reader.get_files()?;
        let img = reader.extract_file(&files[1])?; // Uncompressed Image
        let expected = std::fs::read(expected_path)?;
        assert_eq!(img, expected);
        Ok(())
    }

    #[test]
    fn extract_sample_text_compressed() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData-Encrypted.cpk";
        let expected_path = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData/Text-Compressed.txt";
        if !std::fs::exists(sample_path)? || !std::fs::exists(expected_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new(BufReader::new(File::open(sample_path)?))?;
        let files = reader.get_files()?;
        let text = reader.extract_file(&files[2])?; // Compressed Text
        let expected = std::fs::read(expected_path)?;
        assert_eq!(text, expected);
        Ok(())
    }

    // Persona 5 Royal CPK read tests:
    // Compression + Table Encryption + XOR Scrambling for each file

    #[test]
    fn get_files_p5r() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/SteamLibrary/steamapps/common/P5R/CPK/BASE.CPK";
        if !std::fs::exists(sample_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new(BufReader::new(File::open(sample_path)?))?;
        let _files = reader.get_files()?;
        // for (i, file) in files.iter().enumerate() {
        //     println!("{}: path: {}/{}, size: 0x{:x}/0x{:x}, ofs: 0x{:x}, user: {}", i, file.directory(), file.file_name(), file.file_size(), file.extract_size(), file.file_offset(), file.user_string());
        // }
        Ok(())
    }

    #[test]
    fn extract_p5r_c0001_002_00() -> Result<(), Box<dyn Error>> {
        let sample_path = "E:/SteamLibrary/steamapps/common/P5R/CPK/BASE.CPK";
        let expected_path = "D:/PERSONA5ROYAL/BASE.CPK/MODEL/CHARACTER/0001/C0001_002_00.GMD";
        if !std::fs::exists(sample_path)? || !std::fs::exists(expected_path)? {
            return Ok(());
        }
        let mut reader = CpkReader::new_p5r(BufReader::new(File::open(sample_path)?))?;
        let files = reader.get_files()?;
        let mut file_lookup = HashMap::new();
        for file in &files {
            file_lookup.insert(format!("{}/{}", file.directory(), file.file_name()), file);
        }
        let joker_persona_5 = file_lookup.get("MODEL/CHARACTER/0001/C0001_002_00.GMD").unwrap();
        let joker_persona_5 = reader.extract_file(joker_persona_5)?;
        // std::fs::write("E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/SampleData/joker.GMD", joker_persona_5)?;
        let expected = std::fs::read(expected_path)?;
        assert_eq!(joker_persona_5, expected);
        Ok(())
    }
}