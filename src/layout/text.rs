use crate::config::LayoutConfig;
use crate::metrics::TextMetrics;
use crate::text_metrics;
use crate::theme::Theme;
use crate::unicode_width::{Cluster, consume_cluster, is_cjk_wide_char};

use super::TextBlock;

pub(super) fn measure_label(text: &str, theme: &Theme, config: &LayoutConfig) -> TextBlock {
    // Mermaid's layout sizing appears to use a baseline font size (~16px)
    // even when the configured theme font size is smaller. Using that
    // baseline improves parity with mermaid-cli node sizes.
    let measure_font_size = theme.font_size.max(16.0);
    measure_label_with_font_size(
        text,
        measure_font_size,
        config,
        true,
        theme.font_family.as_str(),
    )
}

pub(super) fn measure_label_with_font_size(
    text: &str,
    font_size: f32,
    config: &LayoutConfig,
    wrap: bool,
    font_family: &str,
) -> TextBlock {
    let fast_metrics = config.fast_text_metrics;
    let max_width_px = max_label_width_px(
        config.max_label_width_chars,
        font_size,
        font_family,
        fast_metrics,
        config.metrics.as_deref(),
    );
    measure_label_with_max_width(text, font_size, max_width_px, config, wrap, font_family)
}

pub(super) fn measure_label_with_max_width(
    text: &str,
    font_size: f32,
    max_width: f32,
    config: &LayoutConfig,
    wrap: bool,
    font_family: &str,
) -> TextBlock {
    let raw_lines = split_lines(text);
    let mut lines = Vec::new();
    let fast_metrics = config.fast_text_metrics;
    let metrics = config.metrics.as_deref();
    // A host's measurement may legitimately be narrower than a character.
    let max_width = max_width.max(0.0);
    for line in raw_lines {
        if wrap {
            let wrapped = wrap_line(
                &line,
                max_width,
                font_size,
                font_family,
                fast_metrics,
                metrics,
            );
            lines.extend(wrapped);
        } else {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
    let measured_width = lines
        .iter()
        .map(|line| text_width(line, font_size, font_family, fast_metrics, metrics))
        .fold(0.0, f32::max);
    let avg_char = average_char_width(font_family, font_size, fast_metrics, metrics);
    let guard_width = max_len as f32 * avg_char;
    // Keep the historical guard against under-measured font metrics. The
    // wrapping fix lives in `wrap_line`: oversized tokens are split before
    // measurement, while an unavoidable single-glyph overflow still reports
    // that glyph's real width here.
    let width = measured_width.max(guard_width);
    let height = lines.len() as f32 * font_size * config.label_line_height;

    TextBlock {
        lines,
        width,
        height,
    }
}

pub(super) fn char_width_factor(ch: char) -> f32 {
    // Calibrated per-character widths against mermaid-cli output using the
    // default font stack and a 16px measurement baseline.
    match ch {
        ' ' => 0.306,
        '\\' | '.' | ',' | ':' | ';' | '|' | '!' | '(' | ')' | '[' | ']' | '{' | '}' => 0.321,
        'A' => 0.652,
        'B' => 0.648,
        'C' => 0.734,
        'D' => 0.723,
        'E' => 0.594,
        'F' => 0.575,
        'G' | 'H' => 0.742,
        'I' => 0.272,
        'J' => 0.557,
        'K' => 0.648,
        'L' => 0.559,
        'M' => 0.903,
        'N' => 0.763,
        'O' => 0.754,
        'P' => 0.623,
        'Q' => 0.755,
        'R' => 0.637,
        'S' => 0.633,
        'T' => 0.599,
        'U' => 0.746,
        'V' => 0.661,
        'W' => 0.958,
        'X' => 0.655,
        'Y' => 0.646,
        'Z' => 0.621,
        'a' => 0.550,
        'b' => 0.603,
        'c' => 0.547,
        'd' => 0.609,
        'e' => 0.570,
        'f' => 0.340,
        'g' | 'h' => 0.600,
        'i' => 0.235,
        'j' => 0.227,
        'k' => 0.522,
        'l' => 0.239,
        'm' => 0.867,
        'n' => 0.585,
        'o' => 0.574,
        'p' => 0.595,
        'q' => 0.585,
        'r' => 0.364,
        's' => 0.523,
        't' => 0.305,
        'u' => 0.585,
        'v' => 0.545,
        'w' => 0.811,
        'x' => 0.538,
        'y' => 0.556,
        'z' => 0.550,
        '0' => 0.613,
        '1' => 0.396,
        '2' => 0.609,
        '3' => 0.597,
        '4' => 0.614,
        '5' => 0.586,
        '6' => 0.608,
        '7' => 0.559,
        '8' => 0.611,
        '9' => 0.595,
        '@' | '#' | '%' | '&' => 0.946,
        _ if is_cjk_wide_char(ch) => 1.0,
        _ => 0.568,
    }
}

pub(super) fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = normalize_display_math(text)
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    current = current.replace("\\n", "\n");
    for line in current.split('\n') {
        lines.push(line.trim().to_string());
    }
    lines
}

fn normalize_display_math(text: &str) -> String {
    if !text.contains("$$") {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("$$") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        if let Some(end) = after_start.find("$$") {
            out.push_str(&render_plain_math(&after_start[..end]));
            rest = &after_start[end + 2..];
        } else {
            out.push_str("$$");
            out.push_str(after_start);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn render_plain_math(input: &str) -> String {
    fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
        if chars.get(open).copied() != Some('{') {
            return None;
        }
        let mut depth = 0usize;
        for (idx, ch) in chars.iter().enumerate().skip(open) {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn read_group(chars: &[char], idx: &mut usize) -> Option<String> {
        while chars.get(*idx).is_some_and(|ch| ch.is_whitespace()) {
            *idx += 1;
        }
        let end = matching_brace(chars, *idx)?;
        let inner: String = chars[*idx + 1..end].iter().collect();
        *idx = end + 1;
        Some(render_plain_math(&inner))
    }

    fn script_char(ch: char, superscript: bool) -> Option<char> {
        let table = if superscript {
            [
                ('0', '⁰'),
                ('1', '¹'),
                ('2', '²'),
                ('3', '³'),
                ('4', '⁴'),
                ('5', '⁵'),
                ('6', '⁶'),
                ('7', '⁷'),
                ('8', '⁸'),
                ('9', '⁹'),
                ('+', '⁺'),
                ('-', '⁻'),
                ('=', '⁼'),
                ('(', '⁽'),
                (')', '⁾'),
            ]
        } else {
            [
                ('0', '₀'),
                ('1', '₁'),
                ('2', '₂'),
                ('3', '₃'),
                ('4', '₄'),
                ('5', '₅'),
                ('6', '₆'),
                ('7', '₇'),
                ('8', '₈'),
                ('9', '₉'),
                ('+', '₊'),
                ('-', '₋'),
                ('=', '₌'),
                ('(', '₍'),
                (')', '₎'),
            ]
        };
        table
            .iter()
            .find_map(|(from, to)| (*from == ch).then_some(*to))
    }

    fn render_script(value: &str, superscript: bool) -> String {
        let mut out = String::new();
        let mut all_mapped = true;
        for ch in value.chars() {
            if let Some(mapped) = script_char(ch, superscript) {
                out.push(mapped);
            } else {
                all_mapped = false;
                break;
            }
        }
        if all_mapped && !out.is_empty() {
            out
        } else if superscript {
            format!("^({})", render_plain_math(value))
        } else {
            format!("_({})", render_plain_math(value))
        }
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut idx = 0usize;
    while idx < chars.len() {
        match chars[idx] {
            '\\' => {
                if chars.get(idx + 1) == Some(&'\\') {
                    out.push_str("; ");
                    idx += 2;
                    continue;
                }
                idx += 1;
                let start = idx;
                while chars.get(idx).is_some_and(|ch| ch.is_ascii_alphabetic()) {
                    idx += 1;
                }
                let command: String = chars[start..idx].iter().collect();
                match command.as_str() {
                    "sqrt" => {
                        if let Some(group) = read_group(&chars, &mut idx) {
                            out.push('√');
                            out.push('(');
                            out.push_str(&group);
                            out.push(')');
                        }
                    }
                    "frac" => {
                        let numerator = read_group(&chars, &mut idx).unwrap_or_default();
                        let denominator = read_group(&chars, &mut idx).unwrap_or_default();
                        out.push('(');
                        out.push_str(&numerator);
                        out.push_str(")/(");
                        out.push_str(&denominator);
                        out.push(')');
                    }
                    "text" => {
                        if let Some(group) = read_group(&chars, &mut idx) {
                            out.push_str(&group);
                        }
                    }
                    "overbrace" => {
                        if let Some(group) = read_group(&chars, &mut idx) {
                            out.push_str(&group);
                        }
                    }
                    "begin" => {
                        let env = read_group(&chars, &mut idx).unwrap_or_default();
                        if env.contains("matrix") {
                            out.push('[');
                        }
                    }
                    "end" => {
                        let env = read_group(&chars, &mut idx).unwrap_or_default();
                        if env.contains("matrix") {
                            out.push(']');
                        }
                    }
                    "pi" => out.push('π'),
                    "alpha" => out.push('α'),
                    "beta" => out.push('β'),
                    "gamma" => out.push('γ'),
                    "delta" => out.push('δ'),
                    "lambda" => out.push('λ'),
                    "mu" => out.push('μ'),
                    "sigma" => out.push('σ'),
                    "theta" => out.push('θ'),
                    "cos" | "sin" | "tan" | "log" | "ln" | "exp" => out.push_str(&command),
                    "left" | "right" | "cdot" => {}
                    _ if !command.is_empty() => out.push_str(&command),
                    _ => out.push('\\'),
                }
            }
            '^' | '_' => {
                let superscript = chars[idx] == '^';
                idx += 1;
                let script = if chars.get(idx) == Some(&'{') {
                    read_group(&chars, &mut idx).unwrap_or_default()
                } else if let Some(ch) = chars.get(idx).copied() {
                    idx += 1;
                    ch.to_string()
                } else {
                    String::new()
                };
                out.push_str(&render_script(&script, superscript));
            }
            '{' | '}' => idx += 1,
            '&' => {
                out.push(' ');
                idx += 1;
            }
            ch => {
                out.push(ch);
                idx += 1;
            }
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn wrap_line(
    line: &str,
    max_width: f32,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> Vec<String> {
    if text_width(line, font_size, font_family, fast_metrics, metrics) <= max_width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };
        if text_width(&candidate, font_size, font_family, fast_metrics, metrics) > max_width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut chunk = String::new();
            for ch in word.chars() {
                let mut candidate = chunk.clone();
                candidate.push(ch);
                if !chunk.is_empty()
                    && text_width(
                        &candidate,
                        font_size,
                        font_family,
                        fast_metrics,
                        metrics,
                    ) > max_width
                {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(ch);
            }
            current = chunk;
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(super) fn text_width(
    text: &str,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> f32 {
    if let Some(width) =
        metrics.and_then(|metrics| metrics.measure_text_width(text, font_size, font_family))
    {
        return width;
    }
    if fast_metrics && text.is_ascii() {
        return fallback_text_width(text, font_size);
    }
    text_metrics::measure_text_width(text, font_size, font_family)
        .unwrap_or_else(|| fallback_text_width(text, font_size))
}

fn fallback_text_width(text: &str, font_size: f32) -> f32 {
    let chars: Vec<char> = text.chars().collect();
    let mut width = 0.0;
    let mut idx = 0usize;
    while idx < chars.len() {
        if let Some((kind, new_idx)) = consume_cluster(&chars, idx) {
            width += match kind {
                Cluster::Wide => 1.0,
                Cluster::ZeroWidth => 0.0,
            };
            idx = new_idx;
            continue;
        }
        width += char_width_factor(chars[idx]);
        idx += 1;
    }
    width * font_size
}

fn average_char_width(
    font_family: &str,
    font_size: f32,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> f32 {
    if let Some(width) =
        metrics.and_then(|metrics| metrics.average_char_width(font_family, font_size))
    {
        return width;
    }
    if fast_metrics {
        return font_size * 0.56;
    }
    text_metrics::average_char_width(font_family, font_size).unwrap_or(font_size * 0.56)
}

fn max_label_width_px(
    max_chars: usize,
    font_size: f32,
    font_family: &str,
    fast_metrics: bool,
    metrics: Option<&dyn TextMetrics>,
) -> f32 {
    let avg_char = average_char_width(font_family, font_size, fast_metrics, metrics);
    (max_chars.max(1) as f32) * avg_char
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lines_handles_br_tags() {
        assert_eq!(split_lines("a<br/>b"), vec!["a", "b"]);
        assert_eq!(split_lines("a<br>b"), vec!["a", "b"]);
        assert_eq!(split_lines("a\\nb"), vec!["a", "b"]);
    }

    #[test]
    fn split_lines_trims_whitespace() {
        assert_eq!(split_lines("  hello  \n  world  "), vec!["hello", "world"]);
    }

    #[test]
    fn char_width_factor_returns_positive_values() {
        for ch in ['a', 'Z', ' ', '0', '@', '\u{4e2d}'] {
            assert!(char_width_factor(ch) > 0.0, "char {:?} has zero width", ch);
        }
    }

    #[test]
    fn char_width_factor_treats_cjk_as_wide() {
        for ch in ['中', 'あ', '한', '。', 'Ａ'] {
            assert_eq!(char_width_factor(ch), 1.0, "char {:?} should be wide", ch);
        }
    }

    #[test]
    fn fallback_text_width_treats_emoji_as_one_em() {
        for text in ["🙂", "🚀", "☀", "❤"] {
            assert_eq!(
                fallback_text_width(text, 16.0),
                16.0,
                "{text} should be 1em via fallback_text_width"
            );
        }
    }

    #[test]
    fn fallback_text_width_counts_emoji_sequences_as_single_wide_glyphs() {
        for text in ["👍🏽", "👨‍👩‍👧‍👦", "🇨🇳", "1️⃣"] {
            assert_eq!(
                fallback_text_width(text, 16.0),
                16.0,
                "{text} should be 1em"
            );
        }
    }

    #[test]
    fn fallback_text_width_scales_with_font_size() {
        let w16 = fallback_text_width("Hello", 16.0);
        let w32 = fallback_text_width("Hello", 32.0);
        assert!(
            (w32 - w16 * 2.0).abs() < 0.01,
            "width should double with font size"
        );
    }

    /// Measures every character as one em, which the built-in estimates never
    /// do, so a width can only come from here.
    #[derive(Debug)]
    struct OneEm;

    impl TextMetrics for OneEm {
        fn measure_text_width(
            &self,
            text: &str,
            font_size: f32,
            _font_family: &str,
        ) -> Option<f32> {
            Some(text.chars().count() as f32 * font_size)
        }
    }

    #[test]
    fn fast_metrics_still_ask_the_host_first() {
        let width = text_width("Hello", 10.0, "serif", true, Some(&OneEm));
        assert_eq!(width, 50.0);
        let average = average_char_width("serif", 10.0, true, Some(&OneEm));
        assert_eq!(average, 10.0);
        // Without a host, the fast estimates stand.
        assert!(text_width("Hello", 10.0, "serif", true, None) < 50.0);
        assert_eq!(average_char_width("serif", 10.0, true, None), 5.6);
    }

    #[test]
    fn wrap_line_does_not_wrap_short_text() {
        let result = wrap_line("short", 1000.0, 16.0, "sans-serif", true, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn wrap_line_splits_long_text() {
        let result = wrap_line(
            "this is a rather long line that should be wrapped",
            100.0,
            16.0,
            "sans-serif",
            true,
            None,
        );
        assert!(result.len() > 1, "expected wrapping, got {:?}", result);
    }

    #[test]
    fn wrap_line_splits_long_ascii_token_into_longest_fitting_chunks() {
        let result = wrap_line("abcdefghij", 30.0, 16.0, "sans-serif", true, None);
        assert_eq!(result.concat(), "abcdefghij");
        assert!(result.len() > 1);
        assert!(
            result
                .iter()
                .all(|line| fallback_text_width(line, 16.0) <= 30.0)
        );
        let mut consumed = 0;
        for line in &result[..result.len() - 1] {
            consumed += line.chars().count();
            let next = "abcdefghij".chars().nth(consumed).unwrap();
            assert!(fallback_text_width(&format!("{line}{next}"), 16.0) > 30.0);
        }
    }

    #[test]
    fn wrap_line_uses_longest_fitting_chunks_for_mixed_width_unicode() {
        let text = "Wi中i界W";
        let max_width = 20.0;
        let result = wrap_line(text, max_width, 16.0, "sans-serif", true, None);
        assert_eq!(result.concat(), text);

        let mut consumed = 0;
        for (index, line) in result.iter().enumerate() {
            assert!(fallback_text_width(line, 16.0) <= max_width);
            consumed += line.chars().count();
            if index + 1 < result.len() {
                let next = text.chars().nth(consumed).unwrap();
                assert!(fallback_text_width(&format!("{line}{next}"), 16.0) > max_width);
            }
        }
    }

    #[test]
    fn wrap_line_splits_unicode_only_at_char_boundaries() {
        let result = wrap_line("你好世界测试", 32.0, 16.0, "sans-serif", true, None);
        assert_eq!(result.concat(), "你好世界测试");
        assert_eq!(result, vec!["你好", "世界", "测试"]);
    }

    #[test]
    fn wrap_line_makes_progress_when_width_is_smaller_than_one_character() {
        let result = wrap_line("wide", 0.1, 16.0, "sans-serif", true, None);
        assert_eq!(result, vec!["w", "i", "d", "e"]);
    }

    #[test]
    fn measure_label_reports_rendered_width_when_limit_is_narrower_than_one_glyph() {
        let config = LayoutConfig {
            fast_text_metrics: true,
            ..LayoutConfig::default()
        };
        let block = measure_label_with_max_width("Wi中", 16.0, 0.1, &config, true, "sans-serif");
        assert_eq!(block.lines, vec!["W", "i", "中"]);
        let rendered_width = block
            .lines
            .iter()
            .map(|line| text_width(line, 16.0, "sans-serif", true, None))
            .fold(0.0, f32::max);
        assert_eq!(block.width, rendered_width);
        assert!(block.width > 0.1);
    }

    #[test]
    fn measured_wrapped_lines_are_contained_by_max_width() {
        let config = LayoutConfig {
            fast_text_metrics: true,
            ..LayoutConfig::default()
        };
        let max_width = 40.0;
        let block = measure_label_with_max_width(
            "averylongunbrokentoken 世界世界世界",
            16.0,
            max_width,
            &config,
            true,
            "sans-serif",
        );
        assert!(block.width <= max_width);
        assert!(
            block
                .lines
                .iter()
                .all(|line| text_width(line, 16.0, "sans-serif", true, None) <= max_width)
        );
    }

    #[test]
    fn measure_label_produces_nonempty_block() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_label("Hello world", &theme, &config);
        assert!(!block.lines.is_empty());
        assert!(block.width > 0.0);
        assert!(block.height > 0.0);
    }

    #[test]
    fn measure_label_empty_string_produces_single_line() {
        let theme = Theme::modern();
        let config = LayoutConfig::default();
        let block = measure_label("", &theme, &config);
        assert_eq!(block.lines.len(), 1);
    }
}
