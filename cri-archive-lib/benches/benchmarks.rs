#![allow(dead_code)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hint::black_box;
use std::io::{BufReader, Read};
// use std::hint::black_box;
use criterion::{ criterion_group, criterion_main, Criterion };
use cri_archive_lib::cpk::compress::layla::{LaylaDecompressor, LaylaDecompressorCursor};
use cri_archive_lib::cpk::encrypt::p5r::P5RDecryptor;
use cri_archive_lib::cpk::encrypt::table::TableDecryptor;
use cri_archive_lib::cpk::file::CpkFile;
use cri_archive_lib::cpk::free_list::FreeList;
use cri_archive_lib::cpk::reader::CpkReader;

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

fn benchmark_layla_decompress(model_data: &[u8], allocator: &mut FreeList) {
    let _ = LaylaDecompressor::decompress(&model_data, allocator);
}

fn decrypt_table_little_init() -> Result<Vec<u8>, Box<dyn Error>> {
    let layla_table = "E:/PersonaMultiplayer/CriFsV2Lib/CriFsV2Lib.Tests/Assets/EncryptedTable.@utf";
    let mut layla_data = vec![];
    File::open(layla_table)?.read_to_end(&mut layla_data)?;
    Ok(layla_data)
}

fn extract_joker_persona5() -> Result<(), Box<dyn Error>> {
    let sample_path = "E:/SteamLibrary/steamapps/common/P5R/CPK/BASE.CPK";
    let expected_path = "D:/PERSONA5ROYAL/BASE.CPK/MODEL/CHARACTER/0001/C0001_002_00.GMD";
    if !std::fs::exists(sample_path)? || !std::fs::exists(expected_path)? {
        return Ok(());
    }
    let mut reader = CpkReader::<_, P5RDecryptor>::new_with_encryption(
        BufReader::new(File::open(sample_path)?))?;
    let files = reader.get_files()?;
    let mut file_lookup = HashMap::new();
    for file in &files {
        file_lookup.insert(format!("{}/{}", file.directory(), file.file_name()), file);
    }
    let joker_persona_5 = file_lookup.get("MODEL/CHARACTER/0001/C0001_002_00.GMD").unwrap();
    let _ = reader.extract_file(joker_persona_5)?;
    Ok(())
}

fn extract_joker_persona5_exclusive(reader: &mut CpkReader<BufReader<File>, P5RDecryptor>, file: &CpkFile) -> Result<(), Box<dyn Error>> {
    let _ = reader.extract_file(file)?;
    Ok(())
}

fn criterion_benchmark(c: &mut Criterion) {
    // let model_data = read_compressed_layla_3d_model().unwrap();
    // let mut allocator = FreeList::new();
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
    /*
    c.bench_function(
        "Layla Decompressor: Decompress", |b| b
            .iter(|| black_box(benchmark_layla_decompress(&model_data, &mut allocator))));
    */
    /*
    let table_little = decrypt_table_little_init().unwrap();
    c.bench_function(
        "Table Decryptor: Little", |b| b
            .iter(|| black_box(TableDecryptor::decrypt_utf(&table_little))));
     */
    /*
    c.bench_function(
        "P5R: Joker Model Test", |b| b
            .iter(|| black_box(extract_joker_persona5())));
     */
    let sample_path = "E:/SteamLibrary/steamapps/common/P5R/CPK/BASE.CPK";
    let mut reader = CpkReader::<_, P5RDecryptor>::new_with_encryption(
        BufReader::new(File::open(sample_path).unwrap())).unwrap();
    let files = reader.get_files().unwrap();
    let mut file_lookup = HashMap::new();
    for file in &files {
        file_lookup.insert(format!("{}/{}", file.directory(), file.file_name()), file);
    }
    let joker_persona_5 = file_lookup.get("MODEL/CHARACTER/0001/C0001_002_00.GMD").unwrap();
    c.bench_function(
        "Joker Persona 5", |b| b
        // "BGM AWB", |b| b
            .iter(|| black_box(extract_joker_persona5_exclusive(&mut reader, joker_persona_5))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);