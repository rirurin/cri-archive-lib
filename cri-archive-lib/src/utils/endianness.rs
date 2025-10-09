pub(crate) struct BigEndian;
// pub(crate) struct LittleEndian;
// pub(crate) struct NativeEndian;

pub(crate) trait Endianness {
    fn get_u16(arr: [u8; 2]) -> u16;
    fn get_u32(arr: [u8; 4]) -> u32;
    fn get_u64(arr: [u8; 8]) -> u64;

    fn get_i16(arr: [u8; 2]) -> i16 {
        Self::get_u16(arr) as i16
    }
    fn get_i32(arr: [u8; 4]) -> i32 {
        Self::get_u32(arr) as i32
    }
    fn get_f32(arr: [u8; 4]) -> f32 {
        Self::get_u32(arr) as f32
    }
    fn get_i64(arr: [u8; 8]) -> i64 {
        Self::get_u64(arr) as i64
    }
    fn get_f64(arr: [u8; 8]) -> f64 {
        Self::get_u64(arr) as f64
    }
}

impl Endianness for BigEndian {
    fn get_u16(arr: [u8; 2]) -> u16 {
        u16::from_be_bytes(arr)
    }
    fn get_u32(arr: [u8; 4]) -> u32 {
        u32::from_be_bytes(arr)
    }
    fn get_u64(arr: [u8; 8]) -> u64 {
        u64::from_be_bytes(arr)
    }
}

/*
impl Endianness for LittleEndian {
    fn get_u16(arr: [u8; 2]) -> u16 {
        u16::from_le_bytes(arr)
    }
    fn get_u32(arr: [u8; 4]) -> u32 {
        u32::from_le_bytes(arr)
    }
    fn get_u64(arr: [u8; 8]) -> u64 {
        u64::from_le_bytes(arr)
    }
}

impl Endianness for NativeEndian {
    fn get_u16(arr: [u8; 2]) -> u16 {
        u16::from_ne_bytes(arr)
    }
    fn get_u32(arr: [u8; 4]) -> u32 {
        u32::from_ne_bytes(arr)
    }
    fn get_u64(arr: [u8; 8]) -> u64 {
        u64::from_ne_bytes(arr)
    }
}
*/