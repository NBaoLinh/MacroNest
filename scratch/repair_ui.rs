use std::fs;

fn cp1252_to_byte(c: char) -> Option<u8> {
    match c {
        '\u{00}'..='\u{7F}' => Some(c as u8),
        '\u{A0}'..='\u{FF}' => Some(c as u8),
        '€' => Some(0x80),
        '‚' => Some(0x82),
        'ƒ' => Some(0x83),
        '„' => Some(0x84),
        '…' => Some(0x85),
        '†' => Some(0x86),
        '‡' => Some(0x87),
        'ˆ' => Some(0x88),
        '‰' => Some(0x89),
        'Š' => Some(0x8A),
        '‹' => Some(0x8B),
        'Œ' => Some(0x8C),
        'Ž' => Some(0x8E),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '–' => Some(0x96),
        '—' => Some(0x97),
        '˜' => Some(0x98),
        '™' => Some(0x99),
        'š' => Some(0x9A),
        '›' => Some(0x9B),
        'œ' => Some(0x9C),
        'ž' => Some(0x9E),
        'Ÿ' => Some(0x9F),
        '\u{81}' => Some(0x81),
        '\u{8D}' => Some(0x8D),
        '\u{8F}' => Some(0x8F),
        '\u{90}' => Some(0x90),
        '\u{9D}' => Some(0x9D),
        _ => None,
    }
}

fn is_high_char(c: char) -> bool {
    c as u32 >= 0x80
}

fn decode_run_once(s: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(s.len());
    for c in s.chars() {
        if let Some(b) = cp1252_to_byte(c) {
            bytes.push(b);
        } else {
            println!("Decoding failed at character: '{}' (U+{:04X}) in run '{}'", c, c as u32, s);
            return None;
        }
    }
    String::from_utf8(bytes).ok()
}

fn fully_decode_run(s: &str) -> String {
    let mut current = s.to_string();
    let mut steps = 0;
    while steps < 6 {
        if let Some(decoded) = decode_run_once(&current) {
            if decoded == current {
                break;
            }
            current = decoded;
            steps += 1;
        } else {
            break;
        }
    }
    current
}

fn repair_literal(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut repaired = String::new();
    let mut i = 0;

    while i < chars.len() {
        if is_high_char(chars[i]) {
            // Find contiguous run of high characters
            let mut run = String::new();
            while i < chars.len() && is_high_char(chars[i]) {
                run.push(chars[i]);
                i += 1;
            }
            // Fully decode this non-ASCII block
            repaired.push_str(&fully_decode_run(&run));
        } else {
            repaired.push(chars[i]);
            i += 1;
        }
    }
    repaired
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "src/ui.rs";
    let content = fs::read_to_string(path)?;
    let mut output = String::with_capacity(content.len());

    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut literal = String::new();
            let mut escaped = false;
            while let Some(&nc) = chars.peek() {
                if escaped {
                    literal.push(nc);
                    chars.next();
                    escaped = false;
                } else if nc == '\\' {
                    literal.push(nc);
                    chars.next();
                    escaped = true;
                } else if nc == '"' {
                    break;
                } else {
                    literal.push(nc);
                    chars.next();
                }
            }
            let repaired = repair_literal(&literal);
            output.push('"');
            output.push_str(&repaired);
            if chars.peek() == Some(&'"') {
                output.push('"');
                chars.next();
            }
        } else {
            output.push(c);
        }
    }

    fs::write(path, output)?;
    println!("Advanced repair completed!");
    Ok(())
}
