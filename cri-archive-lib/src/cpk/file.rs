#[derive(Debug)]
pub struct CpkFile<'a> {
    /// String some developers attach to provide more info on file, e.g. encrypt this file.
    user_string: &'a str,
    /// Directory in which the file is contained.
    directory: &'a str,
    /// Name of the file inside the directory.
    file_name: &'a str,
    /// Offset of the file inside the CPK.
    file_offset: u64,
    /// Size of the file inside the CPK.
    file_size: u32,
    /// Size of the file after it's extracted.
    extract_size: u32
}