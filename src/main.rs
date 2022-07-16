//! Learning how to fingerprint audiofiles

#![warn(
    rust_2018_idioms,
    unused,
    rust_2021_compatibility,
    nonstandard_style,
    future_incompatible,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs
)]

use std::fs::File;
use std::path::{Path, PathBuf};

use chromaprint_rust::Context;
use clap::Parser;
use symphonia::core::audio::{Channels, SampleBuffer};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to audio files
    #[clap()]
    audio_file_path: PathBuf,
}

type BoxedError = Box<dyn std::error::Error>;

fn main() -> Result<(), BoxedError> {
    let args = Args::parse();

    let (mut format, mut decoder) = make_parser(&args.audio_file_path)?;
    let track_id = format.default_track().expect("no default track").id;
    let mut sample_buf = None;
    let mut ctx = make_chroma_ctx(&mut decoder)?;

    loop {
        // Get the next packet from the format reader.
        let packet = match format.next_packet() {
            Ok(data) => data,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(error) => {
                return Err(error.into());
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                if sample_buf.is_none() {
                    let duration = audio_buf.capacity() as u64;
                    let spec = *audio_buf.spec();

                    sample_buf = Some(SampleBuffer::<i16>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);
                    ctx.feed(buf.samples())?;
                }
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    ctx.finish()?;
    print!("{}", ctx.get_fingerprint_base64()?.get()?);

    Ok(())
}

type BoxedDecoder = Box<dyn Decoder>;
type BoxedFormatReader = Box<dyn FormatReader>;
fn make_parser(path: &Path) -> Result<(BoxedFormatReader, BoxedDecoder), BoxedError> {
    let file = Box::new(File::open(path)?);
    let mss = MediaSourceStream::new(file, MediaSourceStreamOptions::default());
    let hint = Hint::new();
    let format_opts: FormatOptions = FormatOptions::default();
    let metadata_opts: MetadataOptions = MetadataOptions::default();
    let decoder_opts: DecoderOptions = DecoderOptions::default();
    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let format = probed.format;
    let track = format.default_track().expect("no default track");
    let decoder = symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;
    Ok((format, decoder))
}

fn make_chroma_ctx(decoder: &mut BoxedDecoder) -> Result<Context, Box<dyn std::error::Error>> {
    let mut ctx = Context::default();
    let channel_count: u16 = decoder
        .codec_params()
        .channels
        .map_or(1, Channels::count)
        .try_into()?;
    let sample_rate = decoder.codec_params().sample_rate.unwrap_or(44100);

    ctx.start(sample_rate, channel_count)?;
    Ok(ctx)
}
