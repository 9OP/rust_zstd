use clap::Parser;
use std::fs;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Source file to decompress
    source: String,

    /// Destination file
    destination: String,

    /// Dump information about frames instead of outputing the result
    #[arg(short, long, default_value_t = false)]
    info: bool,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let bytes = fs::read(args.source)?;

    let decoded = zstd_lib::decode(bytes, args.info)?;
    fs::write(args.destination, decoded)?;
    // {
    // let mut stdout = std::io::stdout().lock();
    // stdout.write_all(decoded.as_slice()).unwrap();
    // }

    Ok(())
}
