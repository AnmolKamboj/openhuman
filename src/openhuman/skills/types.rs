//! Shared tool result types used by the tool and node runtime surfaces.

use serde::{Deserialize, Serialize};

/// Serialized-size ceiling for a pass-through MCP content block before it
/// reaches a model prompt.
///
/// A block kind this build does not model is carried through as JSON rather
/// than dropped, and the generic payload of such a block can be a base64 image
/// or audio — megabytes. Above this ceiling the payload is elided, keeping the
/// block type but not the bytes.
const MAX_LLM_BLOCK_BYTES: usize = 64 * 1024;

/// Result of executing a tool, containing content blocks and error status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// List of content blocks returned by the tool.
    pub content: Vec<ToolContent>,
    /// Indicates if the tool encountered an error during execution.
    #[serde(default)]
    pub is_error: bool,
    /// Optional markdown rendering of the result. When the agent loop
    /// is configured with `prefer_markdown`, this is sent to the LLM
    /// instead of the JSON-serialised content blocks. Mirrors the
    /// `markdownFormatted` field on Composio's backend responses
    /// (see #1165) — markdown is significantly cheaper than JSON in
    /// the model context window.
    #[serde(
        default,
        rename = "markdownFormatted",
        skip_serializing_if = "Option::is_none"
    )]
    pub markdown_formatted: Option<String>,
}

impl ToolResult {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: false,
            markdown_formatted: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: message.into(),
            }],
            is_error: true,
            markdown_formatted: None,
        }
    }

    pub fn json(data: serde_json::Value) -> Self {
        Self {
            content: vec![ToolContent::Json { data }],
            is_error: false,
            markdown_formatted: None,
        }
    }

    /// Construct a successful result that carries both a JSON payload
    /// (for programmatic consumers / debugging) and a markdown rendering
    /// (preferred by the agent loop when `prefer_markdown` is on).
    pub fn success_with_markdown(data: serde_json::Value, markdown: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Json { data }],
            is_error: false,
            markdown_formatted: Some(markdown.into()),
        }
    }

    /// Attach (or replace) the markdown rendering on an existing result.
    pub fn with_markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown_formatted = Some(markdown.into());
        self
    }

    /// Returns the markdown rendering when present and non-empty,
    /// otherwise falls back to [`Self::output`]. Used by the agent loop
    /// when token-saving markdown output is requested.
    pub fn output_for_llm(&self, prefer_markdown: bool) -> String {
        if prefer_markdown {
            if let Some(md) = self.markdown_formatted.as_deref() {
                let trimmed = md.trim();
                if !trimmed.is_empty() {
                    return md.to_string();
                }
            }
        }
        self.output()
    }

    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.as_str()),
                ToolContent::Json { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn output(&self) -> String {
        self.content
            .iter()
            .map(|c| match c {
                ToolContent::Text { text } => text.clone(),
                ToolContent::Json { data } => {
                    serde_json::to_string_pretty(data).unwrap_or_default()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl From<tinymcp_bus::McpToolResult> for ToolResult {
    /// Converts a rendered MCP result into this application's tool result.
    ///
    /// The two shapes are the same by construction — the module's was derived
    /// from this one when the client was extracted — so this is a mapping, not
    /// a translation. It exists because they are now types in different crates,
    /// and a conversion written at each call site would be six chances to get
    /// the error flag the wrong way round.
    fn from(result: tinymcp_bus::McpToolResult) -> Self {
        Self {
            content: result
                .content
                .into_iter()
                .map(|block| match block {
                    tinymcp_bus::McpToolContent::Text { text } => ToolContent::Text { text },
                    tinymcp_bus::McpToolContent::Json { data } => ToolContent::Json { data },
                    // The contract's block enum is `#[non_exhaustive]`. A kind
                    // this build does not model is carried through as its JSON
                    // rather than dropped: a caller can still read it, and
                    // dropping it would lose content a server deliberately sent.
                    other => ToolContent::Json {
                        data: elide_oversized_block(&other),
                    },
                })
                .collect(),
            is_error: result.is_error,
            markdown_formatted: result.markdown_formatted,
        }
    }
}

/// Serializes an unrecognized MCP content block, bounding what a model will
/// see.
///
/// The block is kept whole when it is small, so a future payload a server
/// deliberately sent still reaches the caller. Above [`MAX_LLM_BLOCK_BYTES`]
/// — the base64 image or audio case — the payload is replaced with an elided
/// marker while the block type is retained, so a prompt can still tell what
/// kind of content was dropped.
fn elide_oversized_block(block: &tinymcp_bus::McpToolContent) -> serde_json::Value {
    let value = serde_json::to_value(block).unwrap_or(serde_json::Value::Null);
    let serialized = serde_json::to_string(&value).unwrap_or_default();
    if serialized.len() <= MAX_LLM_BLOCK_BYTES {
        return value;
    }

    let kind = value
        .get("type")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "type": kind,
        "data": format!("[{} bytes elided]", serialized.len()),
    })
}

/// A single content block within a `ToolResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolContent {
    Text { text: String },
    Json { data: serde_json::Value },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_result_success() {
        let r = ToolResult::success("done");
        assert!(!r.is_error);
        assert_eq!(r.text(), "done");
        assert_eq!(r.output(), "done");
    }

    #[test]
    fn tool_result_error() {
        let r = ToolResult::error("failed");
        assert!(r.is_error);
        assert_eq!(r.text(), "failed");
    }

    #[test]
    fn tool_result_json() {
        let r = ToolResult::json(json!({"key": "value"}));
        assert!(!r.is_error);
        assert!(r.text().is_empty()); // text() skips JSON blocks
        assert!(r.output().contains("key"));
    }

    #[test]
    fn tool_result_mixed_content() {
        let r = ToolResult {
            content: vec![
                ToolContent::Text {
                    text: "line1".into(),
                },
                ToolContent::Json {
                    data: json!({"a": 1}),
                },
                ToolContent::Text {
                    text: "line2".into(),
                },
            ],
            is_error: false,
            markdown_formatted: None,
        };
        assert_eq!(r.text(), "line1\nline2");
        let output = r.output();
        assert!(output.contains("line1"));
        assert!(output.contains("line2"));
        assert!(output.contains("\"a\""));
    }

    #[test]
    fn tool_result_serde_roundtrip() {
        let r = ToolResult::success("hello");
        let json = serde_json::to_string(&r).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(!back.is_error);
        assert_eq!(back.text(), "hello");
    }

    #[test]
    fn tool_content_text_serde() {
        let c = ToolContent::Text {
            text: "test".into(),
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        let back: ToolContent = serde_json::from_str(&json).unwrap();
        match back {
            ToolContent::Text { text } => assert_eq!(text, "test"),
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn tool_content_json_serde() {
        let c = ToolContent::Json {
            data: json!({"x": 1}),
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"type\":\"json\""));
        let back: ToolContent = serde_json::from_str(&json).unwrap();
        match back {
            ToolContent::Json { data } => assert_eq!(data["x"], 1),
            _ => panic!("expected Json variant"),
        }
    }

    #[test]
    fn tool_result_empty_content() {
        let r = ToolResult {
            content: vec![],
            is_error: false,
            markdown_formatted: None,
        };
        assert!(r.text().is_empty());
        assert!(r.output().is_empty());
    }

    #[test]
    fn output_for_llm_prefers_markdown_when_requested() {
        let r =
            ToolResult::success_with_markdown(json!({"items": [{"id": 1}, {"id": 2}]}), "- 1\n- 2");
        assert_eq!(r.output_for_llm(true), "- 1\n- 2");
        // When prefer_markdown is false, falls back to JSON pretty-print.
        let raw = r.output_for_llm(false);
        assert!(raw.contains("\"items\""));
    }

    #[test]
    fn output_for_llm_falls_back_to_output_when_markdown_missing() {
        let r = ToolResult::success("plain");
        assert_eq!(r.output_for_llm(true), "plain");
        assert_eq!(r.output_for_llm(false), "plain");
    }

    #[test]
    fn output_for_llm_falls_back_when_markdown_blank() {
        let r = ToolResult::success("plain").with_markdown("   \n  ");
        assert_eq!(r.output_for_llm(true), "plain");
    }

    #[test]
    fn markdown_field_serde_roundtrip() {
        let r = ToolResult::success_with_markdown(json!({"a": 1}), "**a**: 1");
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("markdownFormatted"));
        let back: ToolResult = serde_json::from_str(&s).unwrap();
        assert_eq!(back.markdown_formatted.as_deref(), Some("**a**: 1"));
    }

    #[test]
    fn an_oversized_pass_through_block_is_elided_but_keeps_its_type() {
        // A base64 image or audio block can be megabytes; a model should see
        // what kind of block it was, not the bytes.
        let block = tinymcp_bus::McpToolContent::Json {
            data: json!({"base64": "x".repeat(70 * 1024)}),
        };
        let value = elide_oversized_block(&block);
        assert_eq!(value["type"], "json");
        let marker = value["data"].as_str().expect("an elided marker");
        assert!(marker.contains("bytes elided"), "{marker}");
        assert!(!marker.contains("xxxxx"), "the payload must not survive");
    }

    #[test]
    fn a_small_pass_through_block_is_carried_whole() {
        let block = tinymcp_bus::McpToolContent::Json {
            data: json!({"n": 42}),
        };
        let value = elide_oversized_block(&block);
        assert_eq!(value, json!({"type": "json", "data": {"n": 42}}));
    }
}
