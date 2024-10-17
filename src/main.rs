use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, SupportedStreamConfig};
use std::fs::File;
use std::io::BufWriter;
use std::sync::mpsc::{channel, Receiver, Sender};

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
        let devices = host.input_devices()?;
        find_device(devices, &opt.device_in)
    };
    let device_in = device.expect("failed to find input device");
    println!("Input device: {}", device_in.name()?);

    // detect output device
    let device = if opt.device_in == "default" {
        host.default_output_device()
    } else {
        let devices = host.output_devices()?;
        find_device(devices, &opt.device_out)
    };
    let device_out = device.expect("failed to find output device");
    println!("Output device: {}", device_out.name()?);

    let config = device_in
        .default_input_config()
        .expect("failed to get default input config");
    println!("Default input config: {:?}", config);

    let (send, recv) = channel::<f32>();

    // The WAV file we're recording to.
    let spec = wav_spec_from_config(&config);
    let writer = hound::WavWriter::create(&opt.wav, spec)?;
    let recorder = Recorder {
        wav_writer: writer,
        send,
    };

    // Run the input stream on a separate thread.
    let stream_in = make_input_stream(config.clone(), device_in, recorder)?;
    stream_in.play()?;

    let stream_out = make_output_stream(config, device_out, recv)?;
    stream_out.play()?;

    println!("Listening, press Enter to exit...");
    _ = std::io::stdin().read_line(&mut String::new());

    drop(stream_in);
    drop(stream_out);
    // writer.finalize()?;
    Ok(())
}

fn find_device<D: Iterator<Item = Device>>(devices: D, name: &str) -> Option<Device> {
    let mut names = Vec::new();
    for device in devices {
        if let Ok(device_name) = device.name() {
            if device_name == name {
                return Some(device);
            }
            names.push(device_name);
        }
    }
    println!();
    println!(r#"Device "{name}" not found. Available devices:"#);
    for name in names {
        println!(r#"  "{name}""#);
    }
    println!();
    None
}

fn make_input_stream(
    config: SupportedStreamConfig,
    device: Device,
    mut recorder: Recorder,
) -> anyhow::Result<Stream> {
    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };
    use cpal::SampleFormat::*;
    let stream = match config.sample_format() {
        F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| recorder.read(data),
            err_fn,
            None,
        ),
        sample_format => {
            anyhow::bail!("Unsupported sample format '{sample_format}'")
        }
    };
    Ok(stream?)
}

fn make_output_stream(
    config: SupportedStreamConfig,
    device: Device,
    recv: Receiver<f32>,
) -> anyhow::Result<Stream> {
    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };
    use cpal::SampleFormat::*;
    let stream = match config.sample_format() {
        F32 => device.build_output_stream(
            &config.into(),
            move |output, _: &_| {
                for sample in output.iter_mut() {
                    *sample = recv.recv().unwrap_or_default();
                }
            },
            err_fn,
            None,
        ),
        sample_format => {
            anyhow::bail!("Unsupported sample format '{sample_format}'")
        }
    };
    Ok(stream?)
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    let sample_format = config.sample_format();
    hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: (sample_format.sample_size() * 8) as u16,
        sample_format: if sample_format.is_float() {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        },
    }
}

/// Recorder receives audio from input, writes it into a wav file, and sends it to output.
struct Recorder {
    wav_writer: hound::WavWriter<BufWriter<File>>,
    send: Sender<f32>,
}

impl Recorder {
    fn read(&mut self, input: &[f32]) {
        for &sample in input.iter() {
            // let sample = f32::from_sample(sample);
            self.wav_writer.write_sample(sample).unwrap();
            self.send.send(sample).unwrap();
        }
    }
}
