use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use std::io::{Read, Write};

use crate::model::{MacroGroup, MacroPreset, MacroStep};

const PREFIX_STEP: &str = "MN_STEP:";
const PREFIX_PRESET: &str = "MN_PRESET:";
const PREFIX_GROUP: &str = "MN_GROUP:";

pub fn encode_step(step: &MacroStep) -> Result<String> {
    let json = serde_json::to_vec(step).context("Failed to serialize the step")?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(format!(
        "{PREFIX_STEP}{}",
        URL_SAFE_NO_PAD.encode(compressed)
    ))
}

pub fn decode_step(code: &str) -> Result<MacroStep> {
    let payload = code.trim();
    let encoded = payload
        .strip_prefix(PREFIX_STEP)
        .ok_or_else(|| anyhow::anyhow!("The step code format is invalid"))?;
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Failed to decode the step code")?;
    let mut decoder = DeflateDecoder::new(compressed.as_slice());
    let mut json = Vec::new();
    decoder
        .read_to_end(&mut json)
        .context("Failed to decompress the step code")?;
    let step: MacroStep =
        serde_json::from_slice(&json).context("The step code contents are invalid")?;
    Ok(step)
}

pub fn encode_preset(preset: &MacroPreset) -> Result<String> {
    let json = serde_json::to_vec(preset).context("Failed to serialize the preset")?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(format!(
        "{PREFIX_PRESET}{}",
        URL_SAFE_NO_PAD.encode(compressed)
    ))
}

pub fn decode_preset(code: &str) -> Result<MacroPreset> {
    let payload = code.trim();
    let encoded = payload
        .strip_prefix(PREFIX_PRESET)
        .ok_or_else(|| anyhow::anyhow!("The preset code format is invalid"))?;
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Failed to decode the preset code")?;
    let mut decoder = DeflateDecoder::new(compressed.as_slice());
    let mut json = Vec::new();
    decoder
        .read_to_end(&mut json)
        .context("Failed to decompress the preset code")?;
    let preset: MacroPreset =
        serde_json::from_slice(&json).context("The preset code contents are invalid")?;
    Ok(preset)
}

pub fn encode_group(group: &MacroGroup) -> Result<String> {
    let json = serde_json::to_vec(group).context("Failed to serialize the group")?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(format!(
        "{PREFIX_GROUP}{}",
        URL_SAFE_NO_PAD.encode(compressed)
    ))
}

pub fn decode_group(code: &str) -> Result<MacroGroup> {
    let payload = code.trim();
    let encoded = payload
        .strip_prefix(PREFIX_GROUP)
        .ok_or_else(|| anyhow::anyhow!("The group code format is invalid"))?;
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Failed to decode the group code")?;
    let mut decoder = DeflateDecoder::new(compressed.as_slice());
    let mut json = Vec::new();
    decoder
        .read_to_end(&mut json)
        .context("Failed to decompress the group code")?;
    let group: MacroGroup =
        serde_json::from_slice(&json).context("The group code contents are invalid")?;
    Ok(group)
}
