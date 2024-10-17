use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Sample, Stream, SupportedStreamConfig};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

#[derive(Parser, Debug)]
#[command(version, about = "record and echo audio inputs", long_about = None)]
struct Opt {
    /// The audio device to use for recording
    #[arg(long, default_value = "default")]
    device_in: String,

    /// The audio device to use for playing
    #[arg(long, default_value = "default")]
    device_out: String,

    /// The name of the file where to save audio
    #[arg(long, default_value = "recording.wav")]
    wav: String,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();
    let host = cpal::default_host();

    // detect input device
    let device = if opt.device_in == "default" {
        host.default_input_device()
    } else {
        let mut devices = host.input_devices()?;
        devices.find(|x| x.name().map(|y| y == opt.device_in).unwrap_or(false))
    };
    let device_in = device.expect("failed to find input device");
    println!("Input device: {}", device_in.name()?);

    // detect output device
    let device = if opt.device_in == "default" {
        host.default_output_device()
    } else {
        let mut devices = host.output_devices()?;
        devices.find(|x| x.name().map(|y| y == opt.device_out).unwrap_or(false))
    };
    let device_out = device.expect("failed to find output device");
    println!("Output device: {}", device_out.name()?);

    let config = device_in
        .default_input_config()
        .expect("failed to get default input config");
    println!("Default input config: {:?}", config);

    // The WAV file we're recording to.
    let spec = wav_spec_from_config(&config);
    let writer = hound::WavWriter::create(&opt.wav, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    // Run the input stream on a separate thread.
    let stream = make_stream(config, device_in, writer.clone())?;
    println!("Recording...");
    stream.play()?;

    // Let recording go for roughly three seconds.
    std::thread::sleep(std::time::Duration::from_secs(3));
    drop(stream);
    writer.lock().unwrap().take().unwrap().finalize()?;
    println!("Recording {} complete!", &opt.wav);
    Ok(())
}

fn make_stream(
    config: SupportedStreamConfig,
    device: Device,
    writer: WavWriterHandle,
) -> anyhow::Result<Stream> {
    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };
    use cpal::SampleFormat::*;
    let stream = match config.sample_format() {
        I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, &writer),
            err_fn,
            None,
        ),
        I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, &writer),
            err_fn,
            None,
        ),
        I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, &writer),
            err_fn,
            None,
        ),
        F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, &writer),
            err_fn,
            None,
        ),
        sample_format => {
            anyhow::bail!("Unsupported sample format '{sample_format}'")
        }
    };
    Ok(stream?)
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}
