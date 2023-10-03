extern crate zstd_lib;

use clap::Parser;
use std::{fs, io::Write};
use zstd_lib::frame::FrameIterator;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// File name to decompress
    file_name: String,

    /// Dump information about frames instead of outputing the result
    #[arg(short, long, default_value_t = false)]
    info: bool,
}

fn read_file(file_name: String) -> Vec<u8> {
    match fs::read(file_name) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("{}", err.to_string());
            std::process::exit(2)
        }
    }
}

fn main() {
    let args = Args::parse();

    let bytes = read_file(args.file_name);

    for it in FrameIterator::new(&bytes.as_slice()) {
        match it {
            Ok(v) => {
                if args.info {
                    print!("{:#x?}\n", v);
                    continue;
                }

                let data = v.decode();
                let mut stdout = std::io::stdout().lock();
                stdout.write_all(data.as_slice()).unwrap();
            }
            Err(err) => {
                println!("{}", err.to_string());
                std::process::exit(1)
            }
        }
    }
}
