use serde::{Deserialize, Serialize};

/// Label-based scoring effect.
///
/// Maps label names to score effects. Multiple matching labels compound.
///
/// Example YAML:
/// ```yaml
/// labels:
///   - name: "urgent"
///     effect: "+10"
///   - name: "wip"
///     effect: "x0.5"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabelEffect {
    pub name: String,
    pub effect: String,
}

/// Main scoring configuration.
///
/// Defines how PR scores are calculated. Each factor is optional and can use
/// either addition (`+N`) or multiplication (`xN`) operations.
///
/// Example YAML:
/// ```yaml
/// scoring:
///   base_score: 100
///   age: "+1 per 1h"
///   approvals: "x2 per 1"
///   size:
///     exclude: ["*.lock"]
///     buckets:
///       - { range: "<100", effect: "x5" }
///       - { range: ">=500", effect: "x0.5" }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScoringConfig {
    /// Base score before factors are applied (default: 100.0)
    #[serde(default)]
    pub base_score: Option<f64>,

    /// Age factor: format is "+N per <duration>" or "xN per <duration>"
    /// Example: "+1 per 1h" adds 1 point per hour of age
    #[serde(default)]
    pub age: Option<String>,

    /// Approval factor: effect string applied based on approval count
    /// Format: "+N per 1", "xN per 1", "+N", or "xN"
    /// Example: "+10 per 1" adds 10 points per approval
    #[serde(default)]
    pub approvals: Option<String>,

    /// Size factor: bucket-based with optional file exclusions
    #[serde(default)]
    pub size: Option<SizeConfig>,

    /// Label-based scoring effects (case-insensitive, multiple labels compound)
    /// Example: [{ name: "urgent", effect: "+10" }]
    #[serde(default)]
    pub labels: Option<Vec<LabelEffect>>,

    /// Previously reviewed factor: effect applied when user has reviewed PR
    /// Example: "x0.5" to deprioritize previously-reviewed PRs
    #[serde(default)]
    pub previously_reviewed: Option<String>,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),
            approvals: Some("+10 per 1".to_string()),
            size: Some(SizeConfig {
                exclude: None,
                buckets: vec![
                    SizeBucket {
                        range: "<100".to_string(),
                        effect: "x5".to_string(),
                    },
                    SizeBucket {
                        range: "100-500".to_string(),
                        effect: "x1".to_string(),
                    },
                    SizeBucket {
                        range: ">500".to_string(),
                        effect: "x0.5".to_string(),
                    },
                ],
            }),
            labels: None,
            previously_reviewed: None,
        }
    }
}

/// Size factor configuration.
///
/// Supports file exclusion patterns and size-based buckets.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SizeConfig {
    /// Glob patterns for files to exclude from size calculation
    /// Example: ["*.lock", "package-lock.json"]
    #[serde(default)]
    pub exclude: Option<Vec<String>>,

    /// Size buckets mapping line count ranges to effects
    pub buckets: Vec<SizeBucket>,
}

/// Size factor bucket.
///
/// Maps line count ranges to score effects.
/// Range format: "<N", "<=N", ">N", ">=N", "N-M" (inclusive range)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SizeBucket {
    /// Range expression (e.g., "<100", ">=500", "100-500")
    pub range: String,

    /// Effect on score (e.g., "x5", "x0.5")
    pub effect: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scoring_config() {
        let config = ScoringConfig::default();

        assert_eq!(config.base_score, Some(100.0));
        assert_eq!(config.age, Some("+1 per 1h".to_string()));
        assert_eq!(config.approvals, Some("+10 per 1".to_string()));
        assert!(config.size.is_some());
        assert!(config.labels.is_none());
        assert!(config.previously_reviewed.is_none());
    }

    #[test]
    fn test_scoring_config_serde_roundtrip() {
        let config = ScoringConfig::default();
        let yaml = serde_saphyr::to_string(&config).unwrap();
        let parsed: ScoringConfig = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_partial_scoring_config_parse() {
        let yaml = r#"
base_score: 200
age: "+5 per 1h"
"#;
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(config.base_score, Some(200.0));
        assert_eq!(config.age, Some("+5 per 1h".to_string()));
        assert!(config.approvals.is_none());
        assert!(config.size.is_none());
        assert!(config.labels.is_none());
        assert!(config.previously_reviewed.is_none());
    }

    #[test]
    fn test_full_scoring_config_parse() {
        let yaml = r#"
base_score: 100
age: "+1 per 1h"
approvals: "x2 per 1"
size:
  exclude:
    - "*.lock"
    - "package-lock.json"
  buckets:
    - range: "<100"
      effect: "x5"
    - range: ">=500"
      effect: "x0.5"
"#;
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(config.base_score, Some(100.0));
        assert_eq!(config.age, Some("+1 per 1h".to_string()));
        assert_eq!(config.approvals, Some("x2 per 1".to_string()));

        let size = config.size.unwrap();
        assert_eq!(size.exclude.unwrap().len(), 2);
        assert_eq!(size.buckets.len(), 2);
    }

    #[test]
    fn test_empty_scoring_config_parse() {
        let yaml = "{}";
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        assert!(config.base_score.is_none());
        assert!(config.age.is_none());
        assert!(config.approvals.is_none());
        assert!(config.size.is_none());
        assert!(config.labels.is_none());
        assert!(config.previously_reviewed.is_none());
    }

    #[test]
    fn test_size_config_without_exclude() {
        let yaml = r#"
buckets:
  - range: "<100"
    effect: "x5"
"#;
        let config: SizeConfig = serde_saphyr::from_str(yaml).unwrap();
        assert!(config.exclude.is_none());
        assert_eq!(config.buckets.len(), 1);
    }

    #[test]
    fn test_labels_config_parse() {
        let yaml = r#"
labels:
  - name: "urgent"
    effect: "+10"
  - name: "wip"
    effect: "x0.5"
"#;
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        let labels = config.labels.unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].name, "urgent");
        assert_eq!(labels[0].effect, "+10");
        assert_eq!(labels[1].name, "wip");
        assert_eq!(labels[1].effect, "x0.5");
    }

    #[test]
    fn test_previously_reviewed_config_parse() {
        let yaml = r#"
previously_reviewed: "x0.5"
"#;
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(config.previously_reviewed, Some("x0.5".to_string()));
    }

    #[test]
    fn test_full_config_with_all_factors() {
        let yaml = r#"
base_score: 100
age: "+1 per 1h"
approvals: "x2 per 1"
size:
  buckets:
    - range: "<100"
      effect: "x5"
labels:
  - name: "urgent"
    effect: "+20"
previously_reviewed: "x0.5"
"#;
        let config: ScoringConfig = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(config.base_score, Some(100.0));
        assert_eq!(config.age, Some("+1 per 1h".to_string()));
        assert_eq!(config.approvals, Some("x2 per 1".to_string()));
        assert!(config.size.is_some());
        assert_eq!(config.labels.as_ref().unwrap().len(), 1);
        assert_eq!(config.previously_reviewed, Some("x0.5".to_string()));
    }
}
