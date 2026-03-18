#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTrimMode {
    Preserve,
    Trim,
}

pub(crate) fn normalize_token(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn normalize_token_ascii_lower(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

pub(crate) fn normalize_token_or_default(raw: Option<&str>, default: &str) -> String {
    normalize_token(raw).unwrap_or_else(|| default.to_string())
}

pub(crate) fn normalize_token_ascii_lower_or_default(raw: Option<&str>, default: &str) -> String {
    normalize_token_ascii_lower(raw).unwrap_or_else(|| default.to_string())
}

pub(crate) fn parse_with_default<T, F>(raw: Option<&str>, default: T, parse: F) -> T
where
    T: Copy,
    F: FnMut(&str) -> Option<T>,
{
    normalize_token_ascii_lower(raw)
        .as_deref()
        .and_then(parse)
        .unwrap_or(default)
}

pub(crate) fn parse_bool_like_flag(raw: Option<&str>, default: bool) -> bool {
    if let Some(value) = normalize_token_ascii_lower(raw).as_deref() {
        match value {
            "0" | "false" | "no" | "off" | "disabled" => false,
            "1" | "true" | "yes" | "on" | "enabled" => true,
            _ => default,
        }
    } else {
        default
    }
}

#[must_use]
pub fn first_non_empty_output(
    stdout: &str,
    stderr: &str,
    trim_mode: OutputTrimMode,
) -> Option<String> {
    for raw in [stdout, stderr] {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        return Some(match trim_mode {
            OutputTrimMode::Preserve => raw.to_string(),
            OutputTrimMode::Trim => trimmed.to_string(),
        });
    }
    None
}

#[must_use]
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    let Some((clip_idx, _)) = text.char_indices().nth(max_chars) else {
        return text.to_string();
    };

    let mut out = text[..clip_idx].to_string();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_non_empty_output_prefers_stdout_and_preserves_whitespace() {
        let output = first_non_empty_output("  hello \n", "stderr", OutputTrimMode::Preserve);
        assert_eq!(output.as_deref(), Some("  hello \n"));
    }

    #[test]
    fn first_non_empty_output_can_trim_selected_output() {
        let output = first_non_empty_output("  hello \n", "stderr", OutputTrimMode::Trim);
        assert_eq!(output.as_deref(), Some("hello"));
    }

    #[test]
    fn first_non_empty_output_uses_stderr_when_stdout_is_blank() {
        let output = first_non_empty_output("  \n\t", "  error \n", OutputTrimMode::Trim);
        assert_eq!(output.as_deref(), Some("error"));
    }

    #[test]
    fn first_non_empty_output_returns_none_when_both_outputs_are_blank() {
        let output = first_non_empty_output("  \n\t", "\n  ", OutputTrimMode::Preserve);
        assert_eq!(output, None);
    }

    #[test]
    fn truncate_text_preserves_utf8_char_boundaries() {
        let input = "\u{C548}\u{B155}\u{D558}\u{C138}\u{C694}-hello";
        let clipped = truncate_text(input, 5);
        let expected = format!("{}...", "\u{C548}\u{B155}\u{D558}\u{C138}\u{C694}");
        assert_eq!(clipped, expected);
    }

    #[test]
    fn truncate_text_returns_original_when_input_fits_limit() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn normalize_token_returns_none_for_blank_or_empty() {
        assert_eq!(normalize_token(Some("  ")), None);
        assert_eq!(normalize_token(Some("")), None);
        assert_eq!(normalize_token(None), None);
    }

    #[test]
    fn normalize_token_ascii_lower_trims_and_lowercases() {
        assert_eq!(
            normalize_token_ascii_lower(Some(" HeLLo ")).as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn parse_with_default_uses_mapping_and_falls_back() {
        let mode = parse_with_default(
            Some("  ON "),
            false,
            |value| matches!(value, "on" | "true").then_some(true),
        );
        assert!(mode);

        let fallback = parse_with_default(Some(" unknown "), false, |_| None);
        assert!(!fallback);
    }

    #[test]
    fn parse_bool_like_flag_handles_case_and_default() {
        assert!(parse_bool_like_flag(Some(" TRUE "), false));
        assert!(!parse_bool_like_flag(Some("OFF"), true));
        assert!(parse_bool_like_flag(None, true));
    }
}
