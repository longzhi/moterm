/// Simple URL detector — finds http(s):// URLs in terminal text.
/// Returns (start_col, end_col) pairs for a given line string.

pub fn detect_urls(line: &str) -> Vec<(usize, usize, String)> {
    let mut results = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for http:// or https://
        let remaining: String = chars[i..].iter().collect();
        let start = if remaining.starts_with("https://") || remaining.starts_with("http://") {
            Some(i)
        } else {
            None
        };

        if let Some(start_col) = start {
            let mut end = start_col;
            // Advance to end of URL (stop at whitespace or certain delimiters)
            while end < len {
                let ch = chars[end];
                if ch.is_whitespace() || ch == '"' || ch == '\'' || ch == '>' || ch == '<' {
                    break;
                }
                end += 1;
            }
            // Strip trailing punctuation that's likely not part of URL
            while end > start_col {
                let ch = chars[end - 1];
                if matches!(ch, '.' | ',' | ')' | ']' | ';' | ':' | '!' | '?') {
                    end -= 1;
                } else {
                    break;
                }
            }
            if end > start_col + 8 {
                // At least "http://x"
                let url: String = chars[start_col..end].iter().collect();
                results.push((start_col, end, url));
            }
            i = end;
        } else {
            i += 1;
        }
    }

    results
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(url)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(url)
            .spawn();
    }
}
