use clap::Parser;
use std::{fs, io::Write};
use zstd_lib::decrypt;

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

    let decrypted = decrypt(bytes, args.info)?;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(decrypted.as_slice()).unwrap();

    Ok(())
}
