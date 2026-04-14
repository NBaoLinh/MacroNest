use std::{fs, path::Path};

use anyhow::{Context, Result};
use eframe::egui::IconData;
use image::{ColorType, ImageEncoder, codecs::ico::IcoEncoder};
use tiny_skia::Pixmap;

const APP_ICON_SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#0f172a"/>
      <stop offset="100%" stop-color="#1e293b"/>
    </linearGradient>
    <linearGradient id="glow" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#facc15"/>
      <stop offset="100%" stop-color="#22c55e"/>
    </linearGradient>
  </defs>
  <rect x="6" y="6" width="116" height="116" rx="28" fill="url(#bg)"/>
  <circle cx="64" cy="64" r="28" fill="none" stroke="#334155" stroke-width="8"/>
  <circle cx="64" cy="64" r="9" fill="#f8fafc"/>
  <path d="M64 20 v26 M64 82 v26 M20 64 h26 M82 64 h26" stroke="url(#glow)" stroke-width="10" stroke-linecap="round"/>
  <circle cx="64" cy="64" r="38" fill="none" stroke="url(#glow)" stroke-width="4" opacity="0.9"/>
</svg>
"##;

const APP_ICON_DISABLED_SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#334155"/>
      <stop offset="100%" stop-color="#0f172a"/>
    </linearGradient>
    <linearGradient id="glow" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#9ca3af"/>
      <stop offset="100%" stop-color="#6b7280"/>
    </linearGradient>
  </defs>
  <rect x="6" y="6" width="116" height="116" rx="28" fill="url(#bg)"/>
  <circle cx="64" cy="64" r="28" fill="none" stroke="#475569" stroke-width="8"/>
  <circle cx="64" cy="64" r="9" fill="#e2e8f0"/>
  <path d="M64 20 v26 M64 82 v26 M20 64 h26 M82 64 h26" stroke="url(#glow)" stroke-width="10" stroke-linecap="round"/>
  <circle cx="64" cy="64" r="38" fill="none" stroke="url(#glow)" stroke-width="4" opacity="0.9"/>
</svg>
"##;

pub fn icon_data(size: u32) -> Result<IconData> {
    let pixmap = render_pixmap(size, false)?;
    Ok(IconData {
        rgba: pixmap.data().to_vec(),
        width: pixmap.width(),
        height: pixmap.height(),
    })
}

pub fn ensure_ico_file(path: &Path, size: u32) -> Result<()> {
    ensure_ico_file_variant(path, size, false)
}

pub fn ensure_disabled_ico_file(path: &Path, size: u32) -> Result<()> {
    ensure_ico_file_variant(path, size, true)
}

fn ensure_ico_file_variant(path: &Path, size: u32, disabled: bool) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let pixmap = render_pixmap(size, disabled)?;
    let file = fs::File::create(path)
        .with_context(|| format!("Failed to create icon file {}", path.display()))?;
    let encoder = IcoEncoder::new(file);
    encoder.write_image(
        pixmap.data(),
        pixmap.width(),
        pixmap.height(),
        ColorType::Rgba8.into(),
    )?;
    Ok(())
}

fn render_pixmap(size: u32, disabled: bool) -> Result<Pixmap> {
    let options = resvg::usvg::Options::default();
    let svg = if disabled { APP_ICON_DISABLED_SVG } else { APP_ICON_SVG };
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .context("Failed to parse the embedded icon SVG")?;
    let scale = (size as f32 / tree.size().width()).min(size as f32 / tree.size().height());
    let width = (tree.size().width() * scale).round().max(1.0) as u32;
    let height = (tree.size().height() * scale).round().max(1.0) as u32;
    let mut pixmap = Pixmap::new(width, height).context("Failed to create icon pixmap")?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}
