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

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let bytes = fs::read(args.file_name)?;

    for frame in FrameIterator::new(bytes.as_slice()) {
        if args.info {
            println!("{:#x?}", frame?);
            continue;
        }

        let data = frame?.decode()?;
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(data.as_slice()).unwrap();
    }

    Ok(())
}
