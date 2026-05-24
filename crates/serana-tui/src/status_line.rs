//! Status line configuration with presets.

use serde::{Deserialize, Serialize};

/// A single status line segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusSegment {
    Pi,
    Mode,
    Model,
    Session,
    Hostname,
    Git,
    Workspace,
    Tokens,
    TokenRate,
    ContextPct,
    Cost,
    SessionTime,
    ThinkingLevel,
    Iterations,
}

/// A named preset of ordered status line segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPreset {
    pub name: String,
    pub left_segments: Vec<StatusSegment>,
    pub right_segments: Vec<StatusSegment>,
}

/// Built-in presets.
pub fn builtin_presets() -> Vec<StatusPreset> {
    vec![
        StatusPreset {
            name: "default".into(),
            left_segments: vec![
                StatusSegment::Pi,
                StatusSegment::Model,
                StatusSegment::Mode,
                StatusSegment::Workspace,
                StatusSegment::Git,
                StatusSegment::ContextPct,
                StatusSegment::Cost,
            ],
            right_segments: vec![
                StatusSegment::Session,
                StatusSegment::SessionTime,
                StatusSegment::ThinkingLevel,
                StatusSegment::Iterations,
            ],
        },
        StatusPreset {
            name: "compact".into(),
            left_segments: vec![
                StatusSegment::Model,
                StatusSegment::Mode,
                StatusSegment::Git,
            ],
            right_segments: vec![
                StatusSegment::Session,
                StatusSegment::Cost,
                StatusSegment::ContextPct,
            ],
        },
        StatusPreset {
            name: "minimal".into(),
            left_segments: vec![StatusSegment::Workspace, StatusSegment::Git],
            right_segments: vec![
                StatusSegment::Session,
                StatusSegment::Mode,
                StatusSegment::ContextPct,
            ],
        },
        StatusPreset {
            name: "dev".into(),
            left_segments: vec![
                StatusSegment::Pi,
                StatusSegment::Hostname,
                StatusSegment::Model,
                StatusSegment::Mode,
                StatusSegment::Workspace,
                StatusSegment::Git,
            ],
            right_segments: vec![
                StatusSegment::Session,
                StatusSegment::Tokens,
                StatusSegment::TokenRate,
                StatusSegment::ContextPct,
                StatusSegment::Cost,
                StatusSegment::Iterations,
                StatusSegment::SessionTime,
            ],
        },
        StatusPreset {
            name: "cost-focus".into(),
            left_segments: vec![StatusSegment::Model, StatusSegment::Mode],
            right_segments: vec![
                StatusSegment::Session,
                StatusSegment::Tokens,
                StatusSegment::TokenRate,
                StatusSegment::Cost,
                StatusSegment::SessionTime,
                StatusSegment::Iterations,
            ],
        },
    ]
}

/// Resolve a preset name to its segment groups.
pub fn resolve_preset(name: &str) -> StatusPreset {
    builtin_presets()
        .into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| {
            // Default fallback
            resolve_preset("default")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_presets_count() {
        assert!(builtin_presets().len() >= 5);
    }

    #[test]
    fn test_default_preset_has_all_segments() {
        let preset = resolve_preset("default");
        assert!(preset.left_segments.contains(&StatusSegment::Pi));
        assert!(preset.left_segments.contains(&StatusSegment::Cost));
        assert!(preset.right_segments.contains(&StatusSegment::Session));
    }

    #[test]
    fn test_compact_is_shorter() {
        let def = resolve_preset("default");
        let compact = resolve_preset("compact");
        let def_len = def.left_segments.len() + def.right_segments.len();
        let compact_len = compact.left_segments.len() + compact.right_segments.len();
        assert!(compact_len < def_len);
    }

    #[test]
    fn test_unknown_preset_falls_back() {
        let segs = resolve_preset("nonexistent");
        let def = resolve_preset("default");
        assert_eq!(segs.left_segments.len(), def.left_segments.len());
        assert_eq!(segs.right_segments.len(), def.right_segments.len());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let presets = builtin_presets();
        let json = serde_json::to_string(&presets).unwrap();
        let deserialized: Vec<StatusPreset> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), presets.len());
    }
}
