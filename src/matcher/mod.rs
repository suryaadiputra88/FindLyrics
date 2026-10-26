pub fn clean_title(title: &str) -> String {
    let mut cleaned = title.to_string();

    // Check paren / bracket segments: if they contain keywords like "remaster", "official", "video", "audio", "lyrics", remove them.
    cleaned = remove_matched_delimiters(&cleaned, '(', ')');
    cleaned = remove_matched_delimiters(&cleaned, '[', ']');

    cleaned.trim().to_string()
}

fn remove_matched_delimiters(input: &str, open: char, close: char) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == open {
            let mut inside = String::new();
            let mut found_close = false;
            for inner in chars.by_ref() {
                if inner == close {
                    found_close = true;
                    break;
                }
                inside.push(inner);
            }

            if found_close {
                let inside_lower = inside.to_lowercase();
                let should_remove = inside_lower.contains("remaster")
                    || inside_lower.contains("official")
                    || inside_lower.contains("video")
                    || inside_lower.contains("audio")
                    || inside_lower.contains("lyric")
                    || inside_lower.starts_with("feat.")
                    || inside_lower.starts_with("ft.");

                if !should_remove {
                    result.push(open);
                    result.push_str(&inside);
                    result.push(close);
                }
            } else {
                result.push(open);
                result.push_str(&inside);
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn clean_filename_to_title(filename: &str) -> String {
    let name_without_ext = filename
        .rsplit('.')
        .skip(1)
        .collect::<Vec<&str>>()
        .into_iter()
        .rev()
        .collect::<Vec<&str>>()
        .join(".");
    let name = if name_without_ext.is_empty() {
        filename
    } else {
        &name_without_ext
    };

    if let Some((_, title)) = name.split_once(" - ") {
        clean_title(title)
    } else {
        clean_title(name)
    }
}

pub fn is_duration_within_tolerance(
    actual_secs: f64,
    candidate_secs: f64,
    tolerance_secs: f64,
) -> bool {
    (actual_secs - candidate_secs).abs() <= tolerance_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_title() {
        assert_eq!(
            clean_title("Bohemian Rhapsody (Official Audio)"),
            "Bohemian Rhapsody"
        );
        assert_eq!(clean_title("In The End (Remastered)"), "In The End");
        assert_eq!(clean_title("Yesterday (Remastered 2009)"), "Yesterday");
        assert_eq!(clean_title("Numb (Official Music Video)"), "Numb");
    }

    #[test]
    fn test_clean_filename() {
        assert_eq!(
            clean_filename_to_title("Queen - Bohemian Rhapsody.flac"),
            "Bohemian Rhapsody"
        );
        assert_eq!(
            clean_filename_to_title("Hotel California.mp3"),
            "Hotel California"
        );
    }

    #[test]
    fn test_duration_tolerance() {
        assert!(is_duration_within_tolerance(210.0, 211.5, 3.0));
        assert!(!is_duration_within_tolerance(210.0, 220.0, 3.0));
    }
}
