# chamber

A CLI tool that listens to audio input (microphone), saves the recording to a WAV file, and plays it back on the audio output (speakers).

That's it, that's all it does. I use it to record music from my audio sequencers ([PO-33](https://teenage.engineering/store/po-33/) and [EP-133](https://teenage.engineering/store/ep-133)) and hear what is currently playing.

Features:

1. Works on Linux, even if you have [PipeWire](https://en.wikipedia.org/wiki/PipeWire).
1. Dead simple.
1. You can force a specific audio input or output.
1. Saves in the best audio quality.
1. Single-binary distribution.
1. Blazing fast, with almost no sound delay.
1. Powered by crabs.

## Installation

You'll need [cargo](https://doc.rust-lang.org/cargo/), a Rust package manager.

```bash
cargo install chamber
```

## Usage

Just run it:

```bash
chamber
```

It will start listening, and playing back the audio. By default, the WAV file will be saved into `recording.wav` in the current directory.

Run `chamber --help` to see available flags.
