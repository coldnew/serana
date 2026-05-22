//! Status line configuration with presets.

use serde::{Deserialize, Serialize};

/// A single status line segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusSegment {
    Mode,
    Model,
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
    pub segments: Vec<StatusSegment>,
}

/// Built-in presets.
pub fn builtin_presets() -> Vec<StatusPreset> {
    vec![
        StatusPreset {
            name: "default".into(),
            segments: vec![
                StatusSegment::Mode,
                StatusSegment::Model,
                StatusSegment::Hostname,
                StatusSegment::Git,
                StatusSegment::Workspace,
                StatusSegment::Tokens,
                StatusSegment::TokenRate,
                StatusSegment::ContextPct,
                StatusSegment::Cost,
                StatusSegment::SessionTime,
                StatusSegment::ThinkingLevel,
                StatusSegment::Iterations,
            ],
        },
        StatusPreset {
            name: "compact".into(),
            segments: vec![
                StatusSegment::Mode,
                StatusSegment::Model,
                StatusSegment::Tokens,
                StatusSegment::ContextPct,
                StatusSegment::SessionTime,
            ],
        },
        StatusPreset {
            name: "minimal".into(),
            segments: vec![
                StatusSegment::Mode,
                StatusSegment::Model,
                StatusSegment::SessionTime,
            ],
        },
        StatusPreset {
            name: "dev".into(),
            segments: vec![
                StatusSegment::Mode,
                StatusSegment::Model,
                StatusSegment::Git,
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
            segments: vec![
                StatusSegment::Mode,
                StatusSegment::Model,
                StatusSegment::Tokens,
                StatusSegment::TokenRate,
                StatusSegment::Cost,
                StatusSegment::SessionTime,
                StatusSegment::Iterations,
            ],
        },
    ]
}

/// Resolve a preset name to its segment list.
pub fn resolve_preset(name: &str) -> Vec<StatusSegment> {
    builtin_presets()
        .into_iter()
        .find(|p| p.name == name)
        .map(|p| p.segments)
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
        let segs = resolve_preset("default");
        assert!(segs.contains(&StatusSegment::Mode));
        assert!(segs.contains(&StatusSegment::SessionTime));
        assert!(segs.contains(&StatusSegment::Cost));
    }

    #[test]
    fn test_compact_is_shorter() {
        let def = resolve_preset("default");
        let compact = resolve_preset("compact");
        assert!(compact.len() < def.len());
    }

    #[test]
    fn test_unknown_preset_falls_back() {
        let segs = resolve_preset("nonexistent");
        let def = resolve_preset("default");
        assert_eq!(segs.len(), def.len());
    }

    #[test]
    fn test_serialize_roundtrip() {
        let presets = builtin_presets();
        let json = serde_json::to_string(&presets).unwrap();
        let deserialized: Vec<StatusPreset> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), presets.len());
    }
}
