use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Write};

use crate::model::{MacroGroup, MacroPreset, MacroStep};

const PREFIX_STEP: &str = "MN_STEP:";
const PREFIX_PRESET: &str = "MN_PRESET:";
const PREFIX_GROUP: &str = "MN_GROUP:";

const PREFIX_STEP_V2: &str = "MN2_STEP:";
const PREFIX_PRESET_V2: &str = "MN2_PRESET:";
const PREFIX_GROUP_V2: &str = "MN2_GROUP:";

fn compress_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn decompress_bytes(data: &[u8], kind: &str) -> Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .with_context(|| format!("Failed to decompress the {kind} code"))?;
    Ok(decoded)
}

fn encode_v2<T: Serialize>(value: &T, prefix: &str, kind: &str) -> Result<String> {
    let binary =
        rmp_serde::to_vec(value).with_context(|| format!("Failed to serialize the {kind}"))?;
    let compressed = compress_bytes(&binary)?;
    Ok(format!("{prefix}{}", URL_SAFE_NO_PAD.encode(compressed)))
}

fn decode_v1<T: DeserializeOwned>(encoded: &str, kind: &str) -> Result<T> {
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .with_context(|| format!("Failed to decode the {kind} code"))?;
    let json = decompress_bytes(&compressed, kind)?;
    serde_json::from_slice(&json).with_context(|| format!("The {kind} code contents are invalid"))
}

fn decode_v2<T: DeserializeOwned>(encoded: &str, kind: &str) -> Result<T> {
    let compressed = URL_SAFE_NO_PAD
        .decode(encoded)
        .with_context(|| format!("Failed to decode the {kind} code"))?;
    let binary = decompress_bytes(&compressed, kind)?;
    rmp_serde::from_slice(&binary).with_context(|| format!("The {kind} code contents are invalid"))
}

fn decode_any<T: DeserializeOwned>(
    code: &str,
    v2_prefix: &str,
    v1_prefix: &str,
    kind: &str,
) -> Result<T> {
    let payload = code.trim();
    if let Some(encoded) = payload.strip_prefix(v2_prefix) {
        return decode_v2(encoded, kind);
    }
    let encoded = payload
        .strip_prefix(v1_prefix)
        .ok_or_else(|| anyhow::anyhow!("The {kind} code format is invalid"))?;
    decode_v1(encoded, kind)
}

pub fn encode_step(step: &MacroStep) -> Result<String> {
    encode_v2(step, PREFIX_STEP_V2, "step")
}

pub fn decode_step(code: &str) -> Result<MacroStep> {
    decode_any(code, PREFIX_STEP_V2, PREFIX_STEP, "step")
}

pub fn encode_preset(preset: &MacroPreset) -> Result<String> {
    encode_v2(preset, PREFIX_PRESET_V2, "preset")
}

pub fn decode_preset(code: &str) -> Result<MacroPreset> {
    decode_any(code, PREFIX_PRESET_V2, PREFIX_PRESET, "preset")
}

pub fn encode_group(group: &MacroGroup) -> Result<String> {
    encode_v2(group, PREFIX_GROUP_V2, "group")
}

pub fn decode_group(code: &str) -> Result<MacroGroup> {
    decode_any(code, PREFIX_GROUP_V2, PREFIX_GROUP, "group")
}
