use std::{
    fs::File,
    io::BufReader,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rodio::{Decoder, OutputStreamBuilder, Sink, Source};

use crate::model::AudioClipSettings;

struct PreviewPlayback {
    clip: AudioClipSettings,
    started_at: Instant,
    start_position_ms: u64,
    _stream: rodio::OutputStream,
    sink: Sink,
}

static PREVIEW_PLAYBACK: Lazy<Mutex<Option<PreviewPlayback>>> = Lazy::new(|| Mutex::new(None));

pub fn load_duration_ms(path: &str) -> Result<u64> {
    let decoder = open_decoder(path)?;
    let duration = decoder
        .total_duration()
        .context("Could not determine the audio duration")?;
    Ok(duration.as_millis() as u64)
}

pub fn play_clip_async(clip: AudioClipSettings) {
    let _ = try_play_clip_async(clip);
}

pub fn play_clip_sequence_async(clips: Vec<AudioClipSettings>) {
    let _ = try_play_clip_sequence_async(clips);
}

pub fn try_play_clip_async(clip: AudioClipSettings) -> Result<()> {
    try_play_clip_sequence_async(vec![clip])
}

pub fn try_play_clip_sequence_async(clips: Vec<AudioClipSettings>) -> Result<()> {
    let clips = clips
        .into_iter()
        .filter(|clip| clip.enabled && !clip.file_path.trim().is_empty())
        .collect::<Vec<_>>();
    if clips.is_empty() {
        return Ok(());
    }
    for clip in &clips {
        let path = clip.file_path.trim();
        if !Path::new(path).exists() {
            bail!("Audio file was not found");
        }
    }

    let stream = OutputStreamBuilder::open_default_stream()
        .context("Could not open the default audio output")?;
    let sink = Sink::connect_new(stream.mixer());
    for clip in &clips {
        sink.set_volume(clip.volume.clamp(0.0, 2.0));
        sink.append(clipped_source_from_ms(clip, clip.start_ms)?);
    }
    sink.play();
    thread::spawn(move || {
        sink.sleep_until_end();
        drop(sink);
        drop(stream);
    });
    Ok(())
}

pub fn play_clip_blocking(clip: &AudioClipSettings) -> Result<()> {
    play_clip_sequence_blocking(std::slice::from_ref(clip))
}

pub fn play_clip_sequence_blocking(clips: &[AudioClipSettings]) -> Result<()> {
    let clips = clips
        .iter()
        .filter(|clip| clip.enabled && !clip.file_path.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if clips.is_empty() {
        return Ok(());
    }
    for clip in &clips {
        let path = clip.file_path.trim();
        if !Path::new(path).exists() {
            bail!("Audio file was not found");
        }
    }

    let stream = OutputStreamBuilder::open_default_stream()
        .context("Could not open the default audio output")?;
    let sink = Sink::connect_new(stream.mixer());
    for clip in &clips {
        sink.set_volume(clip.volume.clamp(0.0, 2.0));
        sink.append(clipped_source_from_ms(clip, clip.start_ms)?);
    }
    sink.sleep_until_end();
    Ok(())
}

pub fn toggle_preview(clip: AudioClipSettings) -> Result<bool> {
    let start_ms = clip.start_ms;
    toggle_preview_from_ms(clip, start_ms)
}

pub fn start_preview_from_ms(clip: AudioClipSettings, start_position_ms: u64) -> Result<()> {
    if !clip.enabled || clip.file_path.trim().is_empty() {
        bail!("Choose an audio file first");
    }
    start_preview_from_ms_inner(clip, start_position_ms)?;
    Ok(())
}

pub fn toggle_preview_from_ms(mut clip: AudioClipSettings, start_position_ms: u64) -> Result<bool> {
    if !clip.enabled || clip.file_path.trim().is_empty() {
        bail!("Choose an audio file first");
    }
    clip.enabled = true;
    let start_position_ms =
        start_position_ms.clamp(clip.start_ms, clip.end_ms.max(clip.start_ms + 1));

    let mut playback = PREVIEW_PLAYBACK.lock();
    cleanup_preview(&mut playback);

    if playback.as_ref().is_some_and(|current| {
        current.clip == clip && current.start_position_ms == start_position_ms
    }) {
        if let Some(current) = playback.take() {
            current.sink.stop();
        }
        return Ok(false);
    }

    start_preview_from_ms_inner_locked(playback, clip, start_position_ms)?;
    Ok(true)
}

fn start_preview_from_ms_inner(clip: AudioClipSettings, start_position_ms: u64) -> Result<()> {
    let start_position_ms =
        start_position_ms.clamp(clip.start_ms, clip.end_ms.max(clip.start_ms + 1));
    let mut playback = PREVIEW_PLAYBACK.lock();
    cleanup_preview(&mut playback);
    start_preview_from_ms_inner_locked(playback, clip, start_position_ms)
}

fn start_preview_from_ms_inner_locked(
    mut playback: parking_lot::MutexGuard<'_, Option<PreviewPlayback>>,
    clip: AudioClipSettings,
    start_position_ms: u64,
) -> Result<()> {
    if let Some(current) = playback.take() {
        current.sink.stop();
    }

    let stream = OutputStreamBuilder::open_default_stream()
        .context("Could not open the default audio output")?;
    let sink = Sink::connect_new(stream.mixer());
    sink.set_volume(clip.volume.clamp(0.0, 2.0));
    sink.append(clipped_source_from_ms(&clip, start_position_ms)?);
    sink.play();

    *playback = Some(PreviewPlayback {
        clip,
        started_at: Instant::now(),
        start_position_ms,
        _stream: stream,
        sink,
    });
    Ok(())
}

pub fn stop_preview() {
    if let Some(current) = PREVIEW_PLAYBACK.lock().take() {
        current.sink.stop();
    }
}

pub fn is_previewing(clip: &AudioClipSettings) -> bool {
    let mut playback = PREVIEW_PLAYBACK.lock();
    cleanup_preview(&mut playback);
    playback
        .as_ref()
        .is_some_and(|current| current.clip == *clip)
}

pub fn preview_position_ms(clip: &AudioClipSettings) -> Option<u64> {
    let mut playback = PREVIEW_PLAYBACK.lock();
    cleanup_preview(&mut playback);
    let current = playback.as_ref()?;
    if current.clip != *clip {
        return None;
    }

    let elapsed_ms = current.started_at.elapsed().as_millis() as u64;
    let speed = current.clip.speed.clamp(0.25, 3.0);
    let played_ms = ((elapsed_ms as f32) * speed).round().max(0.0) as u64;
    Some(
        current
            .start_position_ms
            .saturating_add(played_ms)
            .min(current.clip.end_ms.max(current.clip.start_ms + 1)),
    )
}

pub fn load_waveform(path: &str, buckets: usize) -> Result<Vec<f32>> {
    let path = path.trim();
    if path.is_empty() {
        bail!("Choose an audio file first");
    }
    if !Path::new(path).exists() {
        bail!("Audio file was not found");
    }

    let mut decoder = open_decoder(path)?;
    let bucket_count = buckets.max(32);
    let estimated_total_samples = decoder.total_duration().map(|duration| {
        (duration.as_secs_f64() * decoder.sample_rate() as f64 * decoder.channels() as f64).round()
            as usize
    });
    let samples_per_bucket = estimated_total_samples
        .map(|total| (total / bucket_count).max(1))
        .unwrap_or(2048);

    let mut peaks = vec![0.0f32; bucket_count];
    let mut sample_index = 0usize;
    for sample in decoder.by_ref() {
        let bucket = (sample_index / samples_per_bucket).min(bucket_count - 1);
        peaks[bucket] = peaks[bucket].max(sample.abs());
        sample_index += 1;
    }

    if sample_index == 0 {
        return Ok(peaks);
    }

    let peak_max = peaks
        .iter()
        .copied()
        .fold(0.0f32, |best, current| best.max(current));
    if peak_max > 0.0 {
        for peak in &mut peaks {
            *peak /= peak_max;
        }
    }

    Ok(peaks)
}

fn clipped_source_from_ms(
    clip: &AudioClipSettings,
    start_ms: u64,
) -> Result<Box<dyn Source<Item = rodio::Sample> + Send>> {
    let decoder = open_decoder(&clip.file_path)?;
    let start = Duration::from_millis(start_ms.max(clip.start_ms));
    let speed = clip.speed.clamp(0.25, 3.0);
    if clip.end_ms <= start_ms {
        Ok(Box::new(
            decoder
                .skip_duration(start)
                .take_duration(Duration::ZERO)
                .speed(speed),
        ))
    } else if clip.end_ms <= clip.start_ms {
        Ok(Box::new(
            decoder
                .skip_duration(start)
                .take_duration(Duration::ZERO)
                .speed(speed),
        ))
    } else {
        let end = Duration::from_millis(clip.end_ms);
        let length = end.saturating_sub(start);
        Ok(Box::new(
            decoder
                .skip_duration(start)
                .take_duration(length)
                .speed(speed),
        ))
    }
}

fn open_decoder(path: &str) -> Result<Decoder<BufReader<File>>> {
    let file = File::open(path).with_context(|| format!("Failed to open audio file: {path}"))?;
    Decoder::new(BufReader::new(file)).context("Failed to decode the audio file")
}

fn cleanup_preview(playback: &mut Option<PreviewPlayback>) {
    if playback
        .as_ref()
        .is_some_and(|current| current.sink.empty())
    {
        *playback = None;
    }
}
