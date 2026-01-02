//! Defines native intrinsics for x86_64.
//! Rewritten from the original C# code for CriFsV2Lib:
//! https://github.com/Sewer56/CriFsV2Lib/blob/master/CriFsV2Lib/Utilities/Intrinsics.cs

use core::arch::x86_64::{__m128i, __m256i, _mm256_and_si256, _mm256_mullo_epi16, _mm256_or_si256, _mm256_set1_epi16, _mm256_slli_epi16, _mm256_srli_epi16, _mm_and_si128, _mm_mullo_epi16, _mm_or_si128, _mm_set1_epi16, _mm_slli_epi16, _mm_srli_epi16};

/// Multiplies individual bytes for AVX registers.
pub unsafe fn multiply_bytes_avx(a: __m256i, b: __m256i) -> __m256i {
    // Derived from https://stackoverflow.com/questions/8193601/sse-multiplication-16-x-uint8-t
    unsafe {
        let even = _mm256_mullo_epi16(a, b);
        let odd = _mm256_mullo_epi16(_mm256_srli_epi16::<8>(a), _mm256_srli_epi16::<8>(b));
        _mm256_or_si256(_mm256_slli_epi16::<8>(odd), _mm256_and_si256(even, _mm256_set1_epi16(0xff)))
    }
}

// Multiplies individual bytes for SSE registers.
pub unsafe fn multiply_bytes_sse(a: __m128i, b: __m128i) -> __m128i {
    // unpack and multiply
    unsafe {
        let even = _mm_mullo_epi16(a, b);
        let odd = _mm_mullo_epi16(_mm_srli_epi16::<8>(a), _mm_srli_epi16::<8>(b));
        _mm_or_si128(_mm_slli_epi16::<8>(odd), _mm_and_si128(even, _mm_set1_epi16(0xff)))
    }
}