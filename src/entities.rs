//! Decode Mermaid and HTML entities in display text after syntax parsing.

use crate::ir::Graph;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::borrow::Cow;

static ENTITIES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"&(?:#[xX][0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9]+);|#([A-Za-z0-9]+);")
        .expect("constant entity pattern")
});

/// Decode one entity layer, retaining unknown entities and allocations without matches.
fn decode(label: &mut String) {
    // Normalize Mermaid #name;/#123; entities, then decode exactly once. Doing
    // this after parsing preserves escaped syntax; SVG still XML-escapes text.
    let decoded = ENTITIES.replace_all(label, |captures: &Captures<'_>| {
        let original = &captures[0];
        let normalized = match captures.get(1) {
            Some(entity) if entity.as_str().bytes().all(|ch| ch.is_ascii_digit()) => {
                format!("&#{};", entity.as_str())
            }
            Some(entity) => format!("&{};", entity.as_str()),
            None => original.to_string(),
        };
        let decoded = html_escape::decode_html_entities(&normalized);
        if decoded == normalized {
            original.to_string()
        } else {
            decoded.into_owned()
        }
    });
    // Most labels contain no entities. Keep their existing allocation.
    if let Cow::Owned(decoded) = decoded {
        *label = decoded;
    }
}

/// Decode display labels after parsing without changing identifiers or graph topology.
/// Call once per parsed graph; decoding again would expand intentionally escaped entities.
pub(crate) fn decode_labels(graph: &mut Graph) {
    for node in graph.nodes.values_mut() {
        decode(&mut node.label);
    }
    for group in &mut graph.subgraphs {
        decode(&mut group.label);
    }
    for edge in &mut graph.edges {
        for label in [&mut edge.label, &mut edge.start_label, &mut edge.end_label]
            .into_iter()
            .flatten()
        {
            decode(label);
        }
    }
    for note in &mut graph.sequence_notes {
        decode(&mut note.label);
    }
    for note in &mut graph.state_notes {
        decode(&mut note.label);
    }
    for frame in &mut graph.sequence_frames {
        for section in &mut frame.sections {
            if let Some(label) = &mut section.label {
                decode(label);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_one_layer_and_preserves_unknown_entities() {
        for (input, expected) in [
            ("#amp; #lt; #quot; #9829;", "& < \" ♥"),
            ("&#x1F980; &#128512;", "🦀 😀"),
            ("&amp;lt; #amp;lt;", "&lt; &lt;"),
            (
                "#unknown; &unknown; #99999999;",
                "#unknown; &unknown; #99999999;",
            ),
            (
                "Polski: zażółć; Українська; 日本語",
                "Polski: zażółć; Українська; 日本語",
            ),
        ] {
            let mut label = input.to_string();
            decode(&mut label);
            assert_eq!(label, expected, "{input}");
        }
    }
}
