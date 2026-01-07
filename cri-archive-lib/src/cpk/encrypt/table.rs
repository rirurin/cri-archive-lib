//! Implementation rewritten from the original CriFsV2Lib in C#:
//! https://github.com/Sewer56/CriFsV2Lib/blob/master/CriFsV2Lib/Encryption/TableDecryptor.cs

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    __m256i, _mm256_lddqu_si256, _mm256_set1_epi8,
    _mm256_setr_epi8, _mm256_storeu_epi8, _mm256_xor_si256,
    __m128i, _mm_lddqu_si128, _mm_set1_epi8, _mm_setr_epi8,
    _mm_storeu_epi8, _mm_xor_si128
};
#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::{ uint8x16_t, vld1q_s8, vdupq_n_s8, vmulq_s8, vst1q_s8, veorq_s8 };
use std::error::Error;
use std::ptr::copy_nonoverlapping;
use crate::from_slice;
use crate::utils::slice::FromSlice;
use crate::utils::endianness::NativeEndian;
use crate::utils::intrinsics::{ multiply_bytes_avx, multiply_bytes_sse };
// TODO: Vectorized implementation for other architectures (ARM)

#[derive(Debug)]
pub struct TableDecryptor;

impl TableDecryptor {
    #[cfg(target_endian = "little")]
    const ENCRYPT_MAGIC: u32 = 0xF5F39E1F;
    #[cfg(target_endian = "big")]
    const ENCRYPT_MAGIC: u32 = 0x1F9EF3F5;

    pub fn is_encrypted(bytes: &[u8]) -> bool {
        #[cfg(feature = "dangerous")] {
            from_slice!(bytes, u32, NativeEndian) == Self::ENCRYPT_MAGIC
        }
        #[cfg(not(feature = "dangerous"))] {
            Self::is_encrypted_non_dangerous(bytes).map_or(false, |v| v == Self::ENCRYPT_MAGIC)
        }
    }

    #[cfg(not(feature = "dangerous"))]
    fn is_encrypted_non_dangerous(bytes: &[u8]) -> Result<u32, Box<dyn Error>> {
        Ok(from_slice!(bytes, u32, NativeEndian))
    }

    pub fn decrypt_utf(input: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(input.len());
        unsafe { copy_nonoverlapping(input.as_ptr(), result.as_mut_ptr(), result.len()) };
        Self::decrypt_utf_in_place(&mut result);
        result
    }

    pub fn decrypt_utf_in_place(input: &mut [u8]) {
        let xor = 95i8;
        if cfg!(target_arch = "x86_64") {
            if cfg!(target_feature = "avx2") {
                return Self::decrypt_in_place_avx2(input, 0, xor);
            } else if cfg!(target_feature = "sse3") {
                return Self::decrypt_in_place_sse3(input, 0, xor);
            }
        } else if cfg!(all(target_arch = "aarch64", target_feature = "neon")) {
            return Self::decrypt_in_place_neon(input, 0, xor);
        }
        Self::decrypt_in_place_u64(input, 0, xor);
    }

    #[inline(always)]
    #[allow(unused_variables, unused_mut)]
    fn decrypt_in_place_avx2(input: &mut [u8], start: usize, mut xor: i8) {
        #[cfg(target_arch = "x86_64")]
        {
            let multipliers = unsafe { _mm256_setr_epi8(
                1, 21, -71, 45, -79, -123, -23, 29, 97,
                -11, 25, 13, 17, 101, 73, -3, -63, -43,
                121, -19, 113, 69, -87, -35, 33, -75,
                -39, -51, -47, 37, 9, -67
            )};
            for i in start..(input.len() >> 5) {
                unsafe {
                    // multiply many at once
                    let value = _mm256_lddqu_si256(
                        (input.as_ptr() as *const __m256i).add(i));
                    let xor_pattern = _mm256_set1_epi8(xor);
                    let multiplied_xor = multiply_bytes_avx(xor_pattern, multipliers);
                    let result = _mm256_xor_si256(value, multiplied_xor);
                    _mm256_storeu_epi8(input.as_mut_ptr().add(i << 5) as _, result);
                    xor = xor.wrapping_mul(-127i8);
                }
            }
            Self::decrypt_in_place_u8(input, (input.len() >> 5) << 5, xor);
        }
    }

    #[inline(always)]
    #[allow(unused_variables, unused_mut)]
    fn decrypt_in_place_sse3(input: &mut [u8], start: usize, mut xor: i8) {
        #[cfg(target_arch = "x86_64")]
        {
            let multipliers = unsafe { _mm_setr_epi8(
                1, 21, -71, 45, -79, -123, -23, 29, 97,
                -11, 25, 13, 17, 101, 73, -3
            )};
            for i in start..(input.len() >> 4) {
                unsafe {
                    // multiply many at once
                    let value = _mm_lddqu_si128(
                        (input.as_ptr() as *const __m128i).add(i));
                    let xor_pattern = _mm_set1_epi8(xor);
                    let multiplied_xor = multiply_bytes_sse(xor_pattern, multipliers);
                    let result = _mm_xor_si128(value, multiplied_xor);
                    _mm_storeu_epi8(input.as_mut_ptr().add(i << 4) as _, result);
                    xor = xor.wrapping_mul(-63i8);
                }
            }
            Self::decrypt_in_place_u8(input, (input.len() >> 4) << 4, xor);
        }
    }

    #[inline(always)]
    #[allow(unused_variables, unused_mut)]
    fn decrypt_in_place_neon(input: &mut [u8], start: usize, mut xor: i8) {
        #[cfg(target_arch = "aarch64")]
        {
            // Untested. Anyone have a MacBook?
            let multipliers_i8: [i8; 0x10] = [
                1, 21, -71, 45, -79, -123, -23, 29, 97,
                -11, 25, 13, 17, 101, 73, -3
            ];
            let multipliers = unsafe { vld1q_s8(multipliers_i8.as_ptr()) };
            for i in start..(input.len() >> 4) {
                unsafe {
                    // multiply many at once
                    let value = vld1q_s8((input.as_ptr() as _).add(i << 4));
                    let xor_pattern = vdupq_n_s8(xor);
                    let multiplied_xor = vmulq_s8(xor_pattern, multipliers);
                    let result = veorq_s8(value, multiplied_xor);
                    vst1q_s8((input.as_mut_ptr() as _).add(i << 4), result);
                    xor = xor.wrapping_mul(-63i8);
                }
            }
            Self::decrypt_in_place_u8(input, (input.len() >> 4) << 4, xor);
        }
    }

    #[inline(always)]
    fn decrypt_in_place_u64(input: &mut [u8], start: usize, mut xor: i8) {
        for i in start..(input.len() >> 3) {
            unsafe { *(input.as_ptr() as *mut u64).add(i) ^=
                xor as u8 as u64 |
                ((xor.wrapping_mul(21) as u8 as u64) << 0x08) |
                ((xor.wrapping_mul(-71) as u8 as u64) << 0x10) |
                ((xor.wrapping_mul(45) as u8 as u64) << 0x18) |
                ((xor.wrapping_mul(-79) as u8 as u64) << 0x20) |
                ((xor.wrapping_mul(-123) as u8 as u64) << 0x28) |
                ((xor.wrapping_mul(-23) as u8 as u64) << 0x30) |
                ((xor.wrapping_mul(29) as u8 as u64) << 0x38);
            }
            xor = xor.wrapping_mul(97);
        }
        Self::decrypt_in_place_u8(input, (input.len() >> 3) << 3, xor);
    }

    #[inline(always)]
    fn decrypt_in_place_u8(input: &mut [u8], start: usize, mut xor: i8) {
        for i in start..input.len() {
            input[i] ^= xor as u8;
            xor = xor.wrapping_mul(21);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::io::{BufReader, Read};
    use crate::cpk::encrypt::table::TableDecryptor;

    #[test]
    fn is_table_encrypted() -> Result<(), Box<dyn Error>> {
        let decrypted = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/DecyptedTable.@utf";
        let encrypted = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/EncryptedTable.@utf";
        if !std::fs::exists(encrypted)? || !std::fs::exists(decrypted)? {
            return Ok(());
        }
        let mut encrypt_handle = BufReader::new(File::open(encrypted)?);
        let mut encrypt_data = vec![];
        encrypt_handle.read_to_end(&mut encrypt_data)?;
        assert_eq!(TableDecryptor::is_encrypted(&encrypt_data), true);
        let mut decrypt_handle = BufReader::new(File::open(decrypted)?);
        let mut decrypt_data = vec![];
        decrypt_handle.read_to_end(&mut decrypt_data)?;
        assert_eq!(TableDecryptor::is_encrypted(&decrypt_data), false);
        Ok(())
    }

    #[test]
    fn can_decrypt_table() -> Result<(), Box<dyn Error>> {
        let decrypted = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/DecyptedTable.@utf";
        let encrypted = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/EncryptedTable.@utf";
        if !std::fs::exists(encrypted)? || !std::fs::exists(decrypted)? {
            return Ok(());
        }
        let mut encrypt_handle = BufReader::new(File::open(encrypted)?);
        let mut encrypt_data = vec![];
        encrypt_handle.read_to_end(&mut encrypt_data)?;
        let mut decrypt_handle = BufReader::new(File::open(decrypted)?);
        let mut decrypt_data = vec![];
        decrypt_handle.read_to_end(&mut decrypt_data)?;
        let mut encrypt_avx2 = encrypt_data.clone();
        let mut encrypt_sse3 = encrypt_data.clone();
        let mut encrypt_u64 = encrypt_data.clone();
        TableDecryptor::decrypt_in_place_avx2(&mut encrypt_avx2, 0, 95);
        assert_eq!(&encrypt_avx2, &decrypt_data);
        TableDecryptor::decrypt_in_place_sse3(&mut encrypt_sse3, 0, 95);
        assert_eq!(&encrypt_sse3, &decrypt_data);
        TableDecryptor::decrypt_in_place_u64(&mut encrypt_u64, 0, 95);
        assert_eq!(&encrypt_u64, &decrypt_data);
        Ok(())
    }
}