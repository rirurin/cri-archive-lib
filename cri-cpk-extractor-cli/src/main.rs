pub mod error_wrapper;
pub mod progress;
pub mod printerr;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;
use console::Term;
use cri_archive_lib::cpk::encrypt::p5r::P5RDecryptor;
use cri_archive_lib::cpk::reader::CpkReader;
use crate::progress::Progress;

use rayon::prelude::*;
use crate::error_wrapper::ErrorWrapper;
use crate::printerr::PrintErr;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stdout = Term::stdout();
    if args.len() < 2 {
        PrintErr::print_to(&stdout, "Missing a path to the CPK to extract.", "Drag the CPK onto the executable or add the path after the executable's name from the terminal.");
        PrintErr::wait_for_key(&stdout);
        return;
    }
    let input_ext = args[1][args[1].rfind(".").map_or(0, |v| v + 1)..].to_lowercase();
    if input_ext != "cpk" {
        PrintErr::print_to(&stdout, "Wrong file extension.", "The input file should have a CPK file extension.");
        PrintErr::wait_for_key(&stdout);
        return;
    }
    let out_folder = match args.len() == 3 {
        true => Path::new(&args[2]).to_path_buf(),
        false => {
            let inner = Path::new(&args[1]).file_prefix().unwrap();
            Path::new(&args[1]).parent().unwrap().join(inner)
        }
    };
    if let Err(e) = extract(Path::new(&args[1]), out_folder) {
        PrintErr::print_to(&stdout, "Error while extracting:", &e.to_string());
        PrintErr::wait_for_key(&stdout);
    }
}

fn extract<P0: AsRef<Path>, P1: AsRef<Path> + Send + Sync>(input: P0, output: P1) -> Result<(), Box<dyn Error>> {
    let stdout = Term::stdout();

    let col_lightblue = match stdout.features().true_colors_supported() {
        true => console::Style::from_dotted_str("#ADD8E6"),
        false => console::Style::from_dotted_str("45"),
    };
    let col_orchid = match stdout.features().true_colors_supported() {
        true => console::Style::from_dotted_str("#DA70D6"),
        false => console::Style::from_dotted_str("135"),
    };
    println!("Input file: {}", col_lightblue.apply_to(input.as_ref().to_str().unwrap()));
    println!("Output directory: {}", col_orchid.apply_to(output.as_ref().to_str().unwrap()));
    std::fs::create_dir_all(output.as_ref())?;
    let mut cpk = CpkReader::<_, P5RDecryptor>::new_with_encryption(
        BufReader::new(File::open(input)?))?;
    let mut files = cpk.get_files()?;
    files.sort_by(|a, b| a.directory().cmp(b.directory()));
    let mut last_dir_created = None;
    let dir_start = Instant::now();
    for file in &files {
        if let Some(last) = last_dir_created {
            if last == file.directory() { continue; }
        }
        std::fs::create_dir_all(output.as_ref().join(file.directory()))?;
        last_dir_created = Some(file.directory());
    }
    let dir_end = Instant::now().duration_since(dir_start).as_micros() as f64 / 1000.;
    println!("Created directories in {} ms", dir_end);
    files.sort_by(|a, b| b.file_size().cmp(&a.file_size()));
    let progress = Progress::new(&files);
    files.into_par_iter().try_for_each(|f| {
        progress.set_current_file(&f);
        let bytes = cpk.extract_file(&f).map_err(|e| ErrorWrapper::new(e))?;
        std::fs::write(output.as_ref().join(
            format!("{}/{}", f.directory(), f.file_name())), bytes)
            .map_err(|e| ErrorWrapper::new(Box::new(e)))?;
        progress.read_one();
        Ok::<(), ErrorWrapper>(())
    })?;
    let extract_time = progress.get_duration().as_secs_f64();
    let (ex_min, ex_sec) = ((extract_time / 60.).floor(), extract_time % 60.);
    let time_str = match ex_min {
        0. => format!("{} sec", ex_sec),
        v => format!("{} min {} sec", v, ex_sec)
    };
    println!("Extracted files in {}", time_str);
    Ok(())
}