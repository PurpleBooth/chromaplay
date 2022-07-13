use std::io::Read;
use chromaprint_rust::Context;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to audio files
    #[clap()]
    audio_file_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let mut data = Vec::new();
    let mut buf = [0u8; 2];
    let mut f = std::fs::File::open(args.audio_file_path).unwrap();
    while f.read_exact(&mut buf).is_ok() {
        data.push(i16::from_le_bytes(buf));
    }
    
    let mut ctx = Context::default();
    ctx.start(44100, 2).unwrap();
    ctx.feed(&data).unwrap();
    ctx.finish().unwrap();


    print!("{}", ctx.get_fingerprint_base64().unwrap().get().unwrap())
}
