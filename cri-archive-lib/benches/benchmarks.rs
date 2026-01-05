#![allow(dead_code)]

use std::error::Error;
use std::fs::File;
use std::hint::black_box;
use std::io::Read;
// use std::hint::black_box;
use criterion::{ criterion_group, criterion_main, Criterion };
use cri_archive_lib::cpk::compress::layla::{LaylaDecompressor, LaylaDecompressorCursor};
use cri_archive_lib::cpk::encrypt::table::TableDecryptor;

fn read_compressed_layla_3d_model() -> Result<Vec<u8>, Box<dyn Error>> {
    let layla_table = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/Compressed3dModel.crilayla";
    let layla_data = std::fs::read(layla_table)?;
    Ok(layla_data)
}

fn benchmark_layla_read_1(cursor: &mut LaylaDecompressorCursor, model_data: &[u8]) {
    while cursor.get_cdata() as usize > model_data.as_ptr() as usize {
        let _ = cursor.read_1();
    }
}

fn benchmark_layla_read_2(cursor: &mut LaylaDecompressorCursor, model_data: &[u8]) {
    while cursor.get_cdata() as usize > model_data.as_ptr() as usize {
        let _ = cursor.read_2();
    }
}

fn benchmark_layla_read_8(cursor: &mut LaylaDecompressorCursor, model_data: &[u8]) {
    while cursor.get_cdata() as usize > model_data.as_ptr() as usize + 1 {
        let _ = cursor.read_8();
    }
}

fn benchmark_layla_read_max_8(cursor: &mut LaylaDecompressorCursor, model_data: &[u8]) {
    while cursor.get_cdata() as usize > model_data.as_ptr() as usize + 1 {
        let _ = cursor.read_max_8(5);
    }
}

fn benchmark_layla_read_13(cursor: &mut LaylaDecompressorCursor, model_data: &[u8]) {
    while cursor.get_cdata() as usize > model_data.as_ptr() as usize + 1 {
        let _ = cursor.read_13();
    }
}

fn benchmark_layla_decompress(model_data: &[u8]) {
    let _ = LaylaDecompressor::decompress(&model_data);
}

fn decrypt_table_little_init() -> Result<Vec<u8>, Box<dyn Error>> {
    let layla_table = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/EncryptedTable.@utf";
    let mut layla_data = vec![];
    File::open(layla_table)?.read_to_end(&mut layla_data)?;
    Ok(layla_data)
}

fn criterion_benchmark(c: &mut Criterion) {
    let model_data = read_compressed_layla_3d_model().unwrap();
    /*
    c.bench_function(
        "Layla Decompressor: Read 13", |b| b.iter(|| {
            let mut cursor = LaylaDecompressorCursor::new(
                unsafe { model_data.as_ptr().add(model_data.len()) }, 0);
            black_box(benchmark_layla_read_13(&mut cursor, &model_data));
    }));
    */
    /*
    c.bench_function(
        "Layla Decompressor: Read 1", |b| b.iter(|| {
            let mut cursor = LaylaDecompressorCursor::new(
                unsafe { model_data.as_ptr().add(model_data.len()) }, 0);
            black_box(benchmark_layla_read_1(&mut cursor, &model_data));
        }));
     */
    /*
    c.bench_function(
        "Layla Decompressor: Read 2", |b| b.iter(|| {
            let mut cursor = LaylaDecompressorCursor::new(
                unsafe { model_data.as_ptr().add(model_data.len()) }, 0);
            black_box(benchmark_layla_read_2(&mut cursor, &model_data));
        }));
    */
    /*
    c.bench_function(
        "Layla Decompressor: Read 8", |b| b.iter(|| {
            let mut cursor = LaylaDecompressorCursor::new(
                unsafe { model_data.as_ptr().add(model_data.len() - 1) }, 7);
            black_box(benchmark_layla_read_8(&mut cursor, &model_data));
        }));
    */
    /*
    c.bench_function(
        "Layla Decompressor: Read Max 8", |b| b.iter(|| {
            let mut cursor = LaylaDecompressorCursor::new(
                unsafe { model_data.as_ptr().add(model_data.len() - 1) }, 5);
            black_box(benchmark_layla_read_max_8(&mut cursor, &model_data));
        }));
    */
    c.bench_function(
        "Layla Decompressor: Decompress", |b| b
            .iter(|| black_box(benchmark_layla_decompress(&model_data))));
    /*
    let table_little = decrypt_table_little_init().unwrap();
    c.bench_function(
        "Table Decryptor: Little", |b| b
            .iter(|| black_box(TableDecryptor::decrypt_utf(&table_little))));
     */
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);