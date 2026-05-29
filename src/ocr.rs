use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
}

#[cfg(windows)]
pub fn available_ocr_languages() -> Vec<String> {
    use windows::Media::Ocr::OcrEngine;
    match OcrEngine::AvailableRecognizerLanguages() {
        Ok(langs) => langs
            .into_iter()
            .filter_map(|l| l.LanguageTag().ok().map(|t| t.to_string()))
            .collect(),
        Err(_) => vec![],
    }
}

#[cfg(not(windows))]
pub fn available_ocr_languages() -> Vec<String> {
    vec![]
}

#[cfg(windows)]
fn friendly_lang_not_installed_msg(language_code: &str) -> String {
    let lang_name = match language_code {
        "en" | "en-US" => "English",
        "vi"           => "Tiếng Việt",
        "zh-Hans"      => "简体中文 (Simplified Chinese)",
        "zh-Hant"      => "繁體中文 (Traditional Chinese)",
        "ja"           => "日本語 (Japanese)",
        "ko"           => "한국어 (Korean)",
        "fr"           => "Français (French)",
        "de"           => "Deutsch (German)",
        "es"           => "Español (Spanish)",
        "ru"           => "Русский (Russian)",
        "th"           => "ไทย (Thai)",
        other          => other,
    };
    format!(
        "Ngôn ngữ OCR '{}' chưa được cài đặt trên Windows.\n\
        Vui lòng vào Settings → Time & Language → Language & Region → Add a language\n\
        và cài thêm gói Optional features → Basic Typing / OCR cho ngôn ngữ đó.",
        lang_name
    )
}

#[cfg(windows)]
pub fn perform_ocr(rgba_bytes: &[u8], width: u32, height: u32, lang: &str) -> Result<OcrResult> {
    use windows::core::HSTRING;
    use windows::Storage::Streams::{InMemoryRandomAccessStream, DataWriter};
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Globalization::Language;

    if rgba_bytes.is_empty() || width == 0 || height == 0 {
        bail!("Empty image or invalid dimensions");
    }

    let mut w = width;
    let mut h = height;
    let mut rgba_vec = rgba_bytes.to_vec();
    let mut scale_factor = 1;

    // Windows OCR requires width and height of the image to be at least 40 pixels.
    // If the captured region is too small, we upscale it to improve detection accuracy.
    if w < 120 || h < 120 {
        scale_factor = if w < 40 || h < 40 { 4 } else { 2 };
        let new_w = w * scale_factor;
        let new_h = h * scale_factor;
        if let Some(img) = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(w, h, rgba_vec.clone()) {
            let resized_img = image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Triangle);
            rgba_vec = resized_img.into_raw();
            w = new_w;
            h = new_h;
        }
    }

    // Convert RGBA to PNG in memory
    let mut png_bytes = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(w, h, rgba_vec)
            .ok_or_else(|| anyhow::anyhow!("Failed to create ImageBuffer from raw pixels"))?;
        img.write_to(&mut cursor, image::ImageFormat::Png)?;
    }

    // Create InMemoryRandomAccessStream
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(&png_bytes)?;
    writer.StoreAsync()?.get()?;
    writer.FlushAsync()?.get()?;
    
    // Seek to beginning of stream
    stream.Seek(0)?;

    // Create SoftwareBitmap via BitmapDecoder
    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
    let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;

    // Initialize Windows OCR Engine
    let ocr_engine = if lang.trim().is_empty() {
        // Try creating from user preferred languages
        match OcrEngine::TryCreateFromUserProfileLanguages() {
            Ok(engine) => engine,
            Err(_) => {
                // Fallback to English
                let language = Language::CreateLanguage(&HSTRING::from("en-US"))?;
                // Check if English is supported before trying
                if !OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
                    bail!("No OCR language pack is installed on this Windows system. Please go to Settings → Time & Language → Language & Region to install a language with OCR support.");
                }
                match OcrEngine::TryCreateFromLanguage(&language) {
                    Ok(engine) => engine,
                    Err(e) => bail!("Failed to create OCR engine for English: {}", e),
                }
            }
        }
    } else {
        let language_code = lang.trim();
        let language = Language::CreateLanguage(&HSTRING::from(language_code))?;

        // Check if language pack is installed before trying to create engine
        if !OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
            bail!("{}", friendly_lang_not_installed_msg(language_code));
        }

        match OcrEngine::TryCreateFromLanguage(&language) {
            Ok(engine) => engine,
            Err(e) => {
                bail!("{}\n(Details: {})", friendly_lang_not_installed_msg(language_code), e);
            }
        }
    };

    // Recognize text
    let ocr_result_async = ocr_engine.RecognizeAsync(&bitmap)?;
    let ocr_result = ocr_result_async.get()?;

    let text = ocr_result.Text()?.to_string();
    let lines = ocr_result.Lines()?;
    let mut words = Vec::new();

    for line in lines {
        let line_words = line.Words()?;
        for word in line_words {
            let word_text = word.Text()?.to_string();
            let rect = word.BoundingRect()?;
            words.push(OcrWord {
                text: word_text,
                x: rect.X / scale_factor as f32,
                y: rect.Y / scale_factor as f32,
                width: rect.Width / scale_factor as f32,
                height: rect.Height / scale_factor as f32,
            });
        }
    }

    Ok(OcrResult { text, words })
}

#[cfg(not(windows))]
pub fn perform_ocr(_rgba_bytes: &[u8], _width: u32, _height: u32, _lang: &str) -> Result<OcrResult> {
    bail!("OCR is only supported on Windows.");
}
