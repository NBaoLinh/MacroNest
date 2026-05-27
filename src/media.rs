use anyhow::{Context, Result, bail};

use crate::model::RgbaColor;

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoMetadata {
    pub width: i32,
    pub height: i32,
    pub duration_ms: u64,
    pub fps: f64,
}

#[cfg(windows)]
use opencv::{
    core::{Mat, Size},
    imgproc,
    prelude::*,
    videoio::{
        CAP_ANY, CAP_PROP_FPS, CAP_PROP_FRAME_COUNT, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH,
        CAP_PROP_POS_MSEC, VideoCapture,
    },
};

#[cfg(windows)]
pub fn load_video_metadata(path: &str) -> Result<VideoMetadata> {
    let capture =
        VideoCapture::from_file(path, CAP_ANY).with_context(|| format!("Open video: {path}"))?;
    if !capture.is_opened()? {
        bail!("Video could not be opened");
    }

    let width = capture.get(CAP_PROP_FRAME_WIDTH)?.round() as i32;
    let height = capture.get(CAP_PROP_FRAME_HEIGHT)?.round() as i32;
    let fps = capture.get(CAP_PROP_FPS)?;
    let frame_count = capture.get(CAP_PROP_FRAME_COUNT)?;
    let duration_ms = if fps.is_finite() && fps > 0.0 && frame_count.is_finite() && frame_count > 0.0 {
        ((frame_count / fps) * 1000.0).round().max(0.0) as u64
    } else {
        0
    };

    Ok(VideoMetadata {
        width,
        height,
        duration_ms,
        fps,
    })
}

#[cfg(not(windows))]
pub fn load_video_metadata(_path: &str) -> Result<VideoMetadata> {
    bail!("Video playback is only supported on Windows")
}

#[cfg(windows)]
pub fn open_video_capture(path: &str, start_ms: u64) -> Result<(VideoCapture, VideoMetadata)> {
    let metadata = load_video_metadata(path)?;
    let mut capture =
        VideoCapture::from_file(path, CAP_ANY).with_context(|| format!("Open video: {path}"))?;
    if !capture.is_opened()? {
        bail!("Video could not be opened");
    }
    if start_ms > 0 {
        let _ = capture.set(CAP_PROP_POS_MSEC, start_ms as f64);
    }
    Ok((capture, metadata))
}

#[cfg(not(windows))]
pub fn open_video_capture(_path: &str, _start_ms: u64) -> Result<((), VideoMetadata)> {
    bail!("Video playback is only supported on Windows")
}

#[cfg(windows)]
pub fn frame_to_premultiplied_bgra(
    frame: &Mat,
    target_width: i32,
    target_height: i32,
    chroma_key: Option<(RgbaColor, u8)>,
) -> Result<Vec<u8>> {
    if target_width <= 0 || target_height <= 0 {
        bail!("Invalid output size");
    }

    let mut resized = Mat::default();
    let source_size = frame.size()?;
    if source_size.width != target_width || source_size.height != target_height {
        imgproc::resize(
            frame,
            &mut resized,
            Size::new(target_width, target_height),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )?;
    } else {
        resized = frame.try_clone()?;
    }

    let mut bgra = Mat::default();
    match resized.channels() {
        4 => bgra = resized,
        3 => imgproc::cvt_color(&resized, &mut bgra, imgproc::COLOR_BGR2BGRA, 0)?,
        1 => imgproc::cvt_color(&resized, &mut bgra, imgproc::COLOR_GRAY2BGRA, 0)?,
        channels => bail!("Unsupported frame channels: {channels}"),
    }

    if !bgra.is_continuous() {
        bgra = bgra.try_clone()?;
    }

    let pixels = bgra.data_bytes_mut()?;
    let tolerance = chroma_key.map(|(_, tol)| tol as i32).unwrap_or(0);
    for chunk in pixels.chunks_exact_mut(4) {
        let b = chunk[0];
        let g = chunk[1];
        let r = chunk[2];
        let mut alpha = chunk[3];

        if let Some((key, _)) = chroma_key {
            let dr = (r as i32 - key.r as i32).abs();
            let dg = (g as i32 - key.g as i32).abs();
            let db = (b as i32 - key.b as i32).abs();
            if dr <= tolerance && dg <= tolerance && db <= tolerance {
                alpha = 0;
            }
        }

        chunk[3] = alpha;
        chunk[0] = ((b as u32 * alpha as u32) / 255) as u8;
        chunk[1] = ((g as u32 * alpha as u32) / 255) as u8;
        chunk[2] = ((r as u32 * alpha as u32) / 255) as u8;
    }

    Ok(pixels.to_vec())
}

#[cfg(not(windows))]
pub fn frame_to_premultiplied_bgra(
    _frame: &(),
    _target_width: i32,
    _target_height: i32,
    _chroma_key: Option<(RgbaColor, u8)>,
) -> Result<Vec<u8>> {
    bail!("Video playback is only supported on Windows")
}
