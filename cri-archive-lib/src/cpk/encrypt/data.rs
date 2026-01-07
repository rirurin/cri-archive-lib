use crate::cpk::file::CpkFile;

pub trait FileDecryptor {
    /// Check if a given file is encrypted. This can be determined using
    /// the file info or from the byte stream
    fn is_encrypted(file: &CpkFile, stream: &[u8]) -> bool;

    /// Decrypts the input by overwriting it
    fn decrypt_in_place(input: &mut [u8]);
}

pub struct DummyDecryptor;

impl FileDecryptor for DummyDecryptor {
    fn is_encrypted(_: &CpkFile, _: &[u8]) -> bool {
        false
    }

    fn decrypt_in_place(_: &mut [u8]) {}
}