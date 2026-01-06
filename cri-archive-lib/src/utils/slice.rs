use crate::utils::endianness::Endianness;

pub(crate) trait FromSlice where Self: Sized {
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self;
}

impl FromSlice for u8 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        slice[offset]
    }

    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        unsafe { *slice.as_ptr().add(offset) }
    }
}

// Dangerous removes any bounds checking on the slice: this results in a single
// mov + bswap at most

impl FromSlice for u16 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u16(TryInto::<[u8; 2]>::try_into(&slice[offset..2 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u16(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 2]) })
    }
}

impl FromSlice for i16 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i16(TryInto::<[u8; 2]>::try_into(&slice[offset..2 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i16(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 2]) })
    }
}

impl FromSlice for u32 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u32(TryInto::<[u8; 4]>::try_into(&slice[offset..4 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u32(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 4]) })
    }
}

impl FromSlice for i32 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i32(TryInto::<[u8; 4]>::try_into(&slice[offset..4 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i32(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 4]) })
    }
}

impl FromSlice for f32 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_f32(TryInto::<[u8; 4]>::try_into(&slice[offset..4 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_f32(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 4]) })
    }
}

impl FromSlice for u64 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u64(TryInto::<[u8; 8]>::try_into(&slice[offset..8 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_u64(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 8]) })
    }
}

impl FromSlice for i64 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i64(TryInto::<[u8; 8]>::try_into(&slice[offset..8 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_i64(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 8]) })
    }
}

impl FromSlice for f64 {
    #[cfg(not(feature = "dangerous"))]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_f64(TryInto::<[u8; 8]>::try_into(&slice[offset..8 + offset]).unwrap())
    }
    #[cfg(feature = "dangerous")]
    fn from_slice<E: Endianness>(slice: &[u8], offset: usize) -> Self {
        E::get_f64(unsafe { *(slice.as_ptr().add(offset) as *const [u8; 8]) })
    }
}

#[macro_export]
macro_rules! from_slice {
    ($var:expr, $ty:ty, $en:ty, $ofs:literal) => {
        <$ty>::from_slice::<$en>($var, $ofs)
    };
    ($var:expr, $ty:ty, $ofs:literal) => {
        from_slice!($var, $ty, BigEndian, $ofs)
    };
    ($var:expr, $ty:ty) => {
        from_slice!($var, $ty, BigEndian, 0)
    };
    ($var:expr, $ty:ty, $en:ty) => {
        from_slice!($var, $ty, $en, 0)
    };
}