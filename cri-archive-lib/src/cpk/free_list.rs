use std::alloc::Layout;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};

const SLAB_SIZE: usize = 64 * 1024 * 1024; // 64 MB
const SLAB_ALIGNMENT: usize = 0x1000; // 4 KB (common page size)
const BLOCK_SHIFT: usize = 0x12; // 256 KB

static BIT_MASK_U8: [u32; 9] = [ 0x00, 0x01, 0x03, 0x07, 0x0f, 0x1f, 0x3f, 0x7f, 0xff ];

trait ListAllocationMethod {
    fn get_free_block_index(list: &FreeList, size: usize) -> usize;
}

struct BasicSlidingWindowAllocator;
impl ListAllocationMethod for BasicSlidingWindowAllocator {
    fn get_free_block_index(list: &FreeList, size: usize) -> usize {
        let mut start = 0;
        loop {
            let occupation = list.check_occupation(start, size);
            if occupation == 0 { break; }
            start += size;
        }
        if start + size <= 255 {
            start
        } else {
            usize::MAX
        }
    }
}

// Allocating is too slow or something like that type beat
#[derive(Debug)]
pub struct FreeList {
    slab: *mut u8,
    used: [u64; 4], // 256 blocks in total, 256 KB each
    lock: AtomicBool
}

impl FreeList {
    fn get_layout_temp() -> Layout {
        // TEMP: allocate 64 MB at start
        unsafe { Layout::from_size_align_unchecked(SLAB_SIZE, SLAB_ALIGNMENT) }
    }

    pub fn new() -> Self {
        Self {
            slab: unsafe { std::alloc::alloc(Self::get_layout_temp()) },
            used: [0; 4],
            lock: AtomicBool::new(false)
        }
    }

    #[allow(dead_code)]
    fn new_without_alloc() -> Self {
        Self {
            slab: std::ptr::null_mut(),
            used: [0; 4],
            lock: AtomicBool::new(false)
        }
    }

    #[inline]
    fn into_block(size: usize) -> usize {
        (size + (1 << BLOCK_SHIFT) - 1) >> BLOCK_SHIFT
    }

    #[inline]
    fn acquire(&mut self) {
        while self.lock.swap(true, Ordering::Acquire) {}
    }

    #[inline]
    fn unacquire(&mut self) {
        self.lock.store(false, Ordering::Release);
    }

    #[inline]
    fn bit_mask_u8(n: usize) -> u32 {
        unsafe { *BIT_MASK_U8.get_unchecked(n) }
    }

    #[inline(always)]
    fn bit_bounds_check(start: usize, len: usize) -> bool {
        len == 0 || start + len > 255
    }

    fn check_occupation(&self, start: usize, mut len: usize) -> u64 {
        if Self::bit_bounds_check(start, len) { return 0; }
        let mut res = 0u64;
        len = len.min(64);
        let mut byte = unsafe { (self.used.as_ptr() as *mut u8).add(start >> 3) };
        let bit = start & 7;
        // Handle mid-bit start
        let mut diff = len.min(8 - bit);
        if bit != 0 {
            unsafe {
                res = (*byte & ((Self::bit_mask_u8(diff) << bit) as u8)) as u64;
                byte = byte.add(1);
                len -= diff;
            }
        }
        // Read bits byte-wise
        for _ in 0..len >> 3 {
            unsafe {
                res = (res << diff) | *byte as u64;
                byte = byte.add(1);
                diff = 8;
            };
        }
        // Handle end bits
        let bit = len & 7;
        if bit != 0 {
            unsafe {
                res = (res << bit) | (*byte & (Self::bit_mask_u8(bit) as u8)) as u64;
            }
        }
        res
    }

    pub(crate) fn bit_on(&mut self, start: usize, mut len: usize) {
        if Self::bit_bounds_check(start, len) { return; }
        let mut byte = unsafe { (self.used.as_ptr() as *mut u8).add(start >> 3) };
        let bit = start & 7;
        // Handle mid-bit start
        // println!("bit_on: len: {}, bit: {}, diff: {}", len, bit, 8 - bit);
        if bit != 0 {
            unsafe {
                let diff = len.min(8 - bit);
                *byte = *byte | ((Self::bit_mask_u8(diff) << bit) as u8);
                byte = byte.add(1);
                len -= diff;
            }
        }
        // Sets bits byte-wise
        for _ in 0..len >> 3 {
            unsafe {
                *byte = 0xff;
                byte = byte.add(1);
            };
        }
        // Handle end bits
        let bit = len & 7;
        if bit != 0 {
            unsafe {
                *byte = *byte | (Self::bit_mask_u8(bit) as u8);
            }
        }
    }

    pub(crate) fn bit_off(&mut self, start: usize, mut len: usize) {
        if Self::bit_bounds_check(start, len) { return; }
        let mut byte = unsafe { (self.used.as_ptr() as *mut u8).add(start >> 3) };
        let bit = start & 7;
        // Handle mid-bit start
        // println!("bit_off: {}, {}", start, len);
        if bit != 0 {
            unsafe {
                let diff = len.min(8 - bit);
                *byte = *byte & !((Self::bit_mask_u8(diff) << bit) as u8);
                byte = byte.add(1);
                len -= diff;
            }
        }
        // Sets bits byte-wise
        for _ in 0..len >> 3 {
            unsafe {
                *byte = 0x0;
                byte = byte.add(1);
            };
        }
        // Handle end bits
        let bit = len & 7;
        if bit != 0 {
            unsafe {
                *byte = *byte & !(Self::bit_mask_u8(bit) as u8);
            }
        }
    }

    /// Allocate into the free list. Returns None if there is not enough space remaining
    pub(crate) fn allocate(&mut self, size: usize) -> FreeListNode {
        let blocks = Self::into_block(size);
        self.acquire();
        let start = BasicSlidingWindowAllocator::get_free_block_index(self, blocks);
        if start == usize::MAX {
            self.unacquire();
            // I guess we'll *have* to allocate then...
            return FreeListNode::new_unmanaged(size);
        }
        self.bit_on(start, blocks);
        self.unacquire();
        let ptr = unsafe { self.slab.add(start << BLOCK_SHIFT) };
        FreeListNode::new_managed(
            ptr, size, unsafe { NonNull::new_unchecked(&raw mut *self) })
    }

    pub(crate) fn deallocate(&mut self, p: &FreeListNode) {
        let blocks = Self::into_block(p.size);
        self.acquire();
        self.bit_off((p.ptr as usize - self.slab as usize) >> BLOCK_SHIFT, blocks);
        self.unacquire();
    }
}

impl Drop for FreeList {
    fn drop(&mut self) {
        if self.slab != std::ptr::null_mut() {
            unsafe { std::alloc::dealloc(self.slab, Self::get_layout_temp()) }
        }
    }
}

#[derive(Debug)]
pub struct FreeListNode {
    ptr: *mut u8,
    size: usize,
    owner: Option<NonNull<FreeList>>
}

impl FreeListNode {

    fn get_layout_static(size: usize) -> Layout {
        unsafe { Layout::from_size_align_unchecked(size, 0x8) }
    }

    fn get_layout(&self) -> Layout {
        Self::get_layout_static(self.size)
    }

    pub(crate) fn new_managed(ptr: *mut u8, size: usize, owner: NonNull<FreeList>) -> Self {
        Self { ptr, size, owner: Some(owner) }
    }

    pub(crate) fn new_unmanaged(size: usize) -> Self {
        Self { ptr: unsafe { std::alloc::alloc(Self::get_layout_static(size)) }, size, owner: None }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.into()
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.into()
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.into()
    }
}

impl From<&FreeListNode> for &[u8] {
    fn from(value: &FreeListNode) -> Self {
        unsafe { std::slice::from_raw_parts(value.ptr, value.size) }
    }
}

impl From<&mut FreeListNode> for &mut [u8] {
    fn from(value: &mut FreeListNode) -> Self {
        unsafe { std::slice::from_raw_parts_mut(value.ptr, value.size) }
    }
}

impl From<FreeListNode> for Vec<u8> {
    fn from(value: FreeListNode) -> Self {
        let mut out = Vec::with_capacity(value.size);
        unsafe {
            out.set_len(out.capacity());
            std::ptr::copy_nonoverlapping(value.ptr, out.as_mut_ptr(), out.len());
        }
        out
    }
}

impl AsRef<[u8]> for FreeListNode {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl PartialEq for FreeListNode {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<Vec<u8>> for FreeListNode {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Drop for FreeListNode {
    fn drop(&mut self) {
        if let Some(mut list) = self.owner {
            unsafe { list.as_mut().deallocate(self) };
        } else {
            unsafe { std::alloc::dealloc(self.ptr, self.get_layout()) }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;
    use crate::cpk::free_list::FreeList;

    #[test]
    fn used_bit_on() -> Result<(), Box<dyn Error>> {
        let mut list = FreeList::new_without_alloc();
        list.bit_on(1, 7);
        assert_eq!(list.used[0], 0xfe);
        list.bit_on(66, 20);
        assert_eq!(list.used[1], 0x3FFFFC);
        list.bit_on(255, 2);
        Ok(())
    }

    #[test]
    fn used_bit_off() -> Result<(), Box<dyn Error>> {
        let mut list = FreeList::new_without_alloc();
        list.bit_on(1, 7);
        list.bit_off(3, 3);
        assert_eq!(list.used[0], 0xc6);
        list.bit_on(66, 20);
        list.bit_off(70, 8);
        assert_eq!(list.used[1], 0x3FC03C);
        Ok(())
    }

    #[test]
    fn used_bit_check() -> Result<(), Box<dyn Error>> {
        let mut list = FreeList::new_without_alloc();
        list.bit_on(1, 7);
        assert_eq!(list.check_occupation(1, 7), 0xfe);
        list.bit_on(66, 20);
        assert_eq!(list.check_occupation(66, 20), 0xFFFFF);
        // Basic search for a blank allocation
        let mut start = 0;
        loop {
            let occ = list.check_occupation(start, 3);
            if occ == 0 { break; }
            start += 3;
        }
        assert_eq!(start, 9);
        Ok(())
    }

    #[test]
    fn list_allocate_basic() -> Result<(), Box<dyn Error>> {
        let mut list = FreeList::new();
        let item1 = list.allocate(0x10);
        assert_eq!(list.used[0], 0x1);
        let item2 = list.allocate(0x10);
        assert_eq!(list.used[0], 0x3);
        let item3 = list.allocate(0xc0000);
        assert_eq!(list.used[0], 0x3b);
        drop(item1);
        assert_eq!(list.used[0], 0x3a);
        drop(item2);
        assert_eq!(list.used[0], 0x38);
        drop(item3);
        assert_eq!(list.used[0], 0x0);
        Ok(())
    }
}