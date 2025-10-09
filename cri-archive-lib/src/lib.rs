#[cfg(feature = "cpk")]
pub mod cpk {

}
pub mod schema {
    pub mod container;
    pub mod columns;
    pub mod header;
    pub mod rows;
    pub mod strings;
}
pub mod utils {
    pub mod endianness;
    pub mod slice;
}