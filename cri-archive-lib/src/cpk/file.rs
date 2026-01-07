use std::ptr::NonNull;

#[derive(Debug)]
pub struct CpkFile {
    /// Directory in which the file is contained. DirName in CRI Table
    directory: NonNull<str>,
    /// Name of the file inside the directory. FileName in CRI Table
    file_name: NonNull<str>,
    /// Offset of the file inside the CPK. FileOffset in CRI Table
    file_offset: u64,
    /// Size of the file inside the CPK. FileSize in CRI Table
    file_size: u32,
    /// Size of the file after it's extracted. ExtractSize in CRI Table
    extract_size: u32,
    /// String some developers attach to provide more info on file, e.g. encrypt this file.
    /// UserString in CRI Table
    user_string: NonNull<str>,
}

impl CpkFile {
    pub fn directory(&self) -> &str { unsafe { self.directory.as_ref() } }
    pub fn file_name(&self) -> &str { unsafe { self.file_name.as_ref() } }
    pub fn file_offset(&self) -> u64 { self.file_offset }
    pub fn file_size(&self) -> u32 { self.file_size }
    pub fn extract_size(&self) -> u32 { self.extract_size }
    pub fn user_string(&self) -> &str { unsafe { self.user_string.as_ref() } }

    pub fn new(directory: &str, file_name: &str, file_offset: u64, file_size: u32,
               extract_size: u32, user_string: &str) -> Self {
        let directory = unsafe { NonNull::new_unchecked(&raw const *directory as *mut str) };
        let file_name = unsafe { NonNull::new_unchecked(&raw const *file_name as *mut str) };
        let user_string = unsafe { NonNull::new_unchecked(&raw const *user_string as *mut str) };
        Self { directory, file_name, file_offset, file_size, extract_size, user_string }
    }
}

unsafe impl Send for CpkFile {}