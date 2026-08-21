//! Stable wire types shared with the separately compiled TinyJuice module.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentTokenjuiceCompression {
    #[default]
    Auto,
    Full,
    Light,
    Off,
}

impl AgentTokenjuiceCompression {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Full => "full",
            Self::Light => "light",
            Self::Off => "off",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Json,
    Code,
    Log,
    Search,
    Diff,
    Html,
    PlainText,
}

impl ContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Code => "code",
            Self::Log => "log",
            Self::Search => "search",
            Self::Diff => "diff",
            Self::Html => "html",
            Self::PlainText => "plain_text",
        }
    }
}

impl std::str::FromStr for ContentKind {
    type Err = ();
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "json" => Self::Json,
            "code" => Self::Code,
            "log" => Self::Log,
            "search" => Self::Search,
            "diff" => Self::Diff,
            "html" => Self::Html,
            "plain_text" => Self::PlainText,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompressorKind {
    SmartCrusher,
    Code,
    Log,
    Search,
    Diff,
    Html,
    MlText,
    Generic,
    None,
}

impl CompressorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SmartCrusher => "smartcrusher",
            Self::Code => "code",
            Self::Log => "log",
            Self::Search => "search",
            Self::Diff => "diff",
            Self::Html => "html",
            Self::MlText => "ml_text",
            Self::Generic => "generic",
            Self::None => "none",
        }
    }
}

impl std::str::FromStr for CompressorKind {
    type Err = ();
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "smartcrusher" => Self::SmartCrusher,
            "code" => Self::Code,
            "log" => Self::Log,
            "search" => Self::Search,
            "diff" => Self::Diff,
            "html" => Self::Html,
            "ml_text" => Self::MlText,
            "generic" => Self::Generic,
            "none" => Self::None,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentHint {
    pub mime: Option<String>,
    pub extension: Option<String>,
    pub source_tool: Option<String>,
    pub query: Option<String>,
    pub explicit: Option<ContentKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompressOptions {
    pub router_enabled: bool,
    pub ccr_enabled: bool,
    pub search_enabled: bool,
    pub code_enabled: bool,
    pub html_enabled: bool,
    pub ml_text_enabled: bool,
    pub min_bytes_to_compress: usize,
    pub min_bytes_to_compress_log: usize,
    pub ccr_min_tokens: usize,
    pub lossy_without_ccr: bool,
    pub max_inline_chars: Option<usize>,
    pub code_target_ratio: Option<f32>,
    pub chars_per_token: f32,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            router_enabled: true,
            ccr_enabled: true,
            search_enabled: true,
            code_enabled: true,
            html_enabled: true,
            ml_text_enabled: false,
            min_bytes_to_compress: 2048,
            min_bytes_to_compress_log: 512,
            ccr_min_tokens: 500,
            lossy_without_ccr: false,
            max_inline_chars: None,
            code_target_ratio: None,
            chars_per_token: 4.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedOutput {
    pub text: String,
    pub content_kind: ContentKind,
    pub compressor: CompressorKind,
    pub lossy: bool,
    pub applied: bool,
    pub ccr_token: Option<String>,
    pub original_bytes: usize,
    pub compacted_bytes: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactResponse {
    pub text: String,
    pub original_bytes: usize,
    pub compacted_bytes: usize,
    pub rule_id: String,
    pub applied: bool,
    pub content_kind: String,
    pub compressor: String,
    pub original_tokens: u64,
    pub compacted_tokens: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeUnit {
    Bytes,
    Lines,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveRange {
    pub start: usize,
    pub end: usize,
    pub unit: RangeUnit,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub entries: usize,
    pub bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_options_accept_omitted_fields() {
        let options: CompressOptions = serde_json::from_value(serde_json::json!({
            "routerEnabled": false
        }))
        .expect("partial options remain forward-compatible");
        assert!(!options.router_enabled);
        assert!(options.ccr_enabled);
        assert_eq!(options.min_bytes_to_compress, 2048);
    }

    #[test]
    fn content_hint_matches_the_module_wire_shape() {
        let hint = ContentHint {
            source_tool: Some("shell".to_string()),
            explicit: Some(ContentKind::PlainText),
            ..ContentHint::default()
        };
        assert_eq!(
            serde_json::to_value(hint).expect("serialize hint"),
            serde_json::json!({
                "mime": null,
                "extension": null,
                "sourceTool": "shell",
                "query": null,
                "explicit": "plainText"
            })
        );
    }

    #[test]
    fn retrieve_range_uses_camel_case_wire_values() {
        let range = RetrieveRange {
            start: 2,
            end: 5,
            unit: RangeUnit::Lines,
        };
        assert_eq!(
            serde_json::to_value(range).expect("serialize range"),
            serde_json::json!({ "start": 2, "end": 5, "unit": "lines" })
        );
    }
}
