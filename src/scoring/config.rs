use serde::{Deserialize, Serialize};

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
///   approvals:
///     - { range: "0", effect: "x0.5" }
///     - { range: ">0", effect: "x2 per 1" }
///   size:
///     exclude: ["*.lock"]
///     buckets:
///       - { range: "<100", effect: "x5" }
///       - { range: ">=500", effect: "x0.5" }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ScoringConfig {
    /// Base score before factors are applied (default: 100.0)
    #[serde(default)]
    pub base_score: Option<f64>,

    /// Age factor: format is "+N per <duration>" or "xN per <duration>"
    /// Example: "+1 per 1h" adds 1 point per hour of age
    #[serde(default)]
    pub age: Option<String>,

    /// Approval factor: bucket-based configuration
    /// Each bucket has a range (e.g., "0", ">0", ">=2") and an effect
    #[serde(default)]
    pub approvals: Option<Vec<ApprovalBucket>>,

    /// Size factor: bucket-based with optional file exclusions
    #[serde(default)]
    pub size: Option<SizeConfig>,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),
            approvals: Some(vec![
                ApprovalBucket {
                    range: "0".to_string(),
                    effect: "x0.5".to_string(),
                },
                ApprovalBucket {
                    range: ">0".to_string(),
                    effect: "x2 per 1".to_string(),
                },
            ]),
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
        }
    }
}

/// Approval factor bucket.
///
/// Maps approval count ranges to score effects.
/// Range operators: <, <=, >, >=, = (or just the number for exact match)
/// Effect format: "+N", "xN", or "xN per M" (per additional approval)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ApprovalBucket {
    /// Range expression (e.g., "0", ">0", ">=2")
    pub range: String,

    /// Effect on score (e.g., "x0.5", "+10", "x2 per 1")
    pub effect: String,
}

/// Size factor configuration.
///
/// Supports file exclusion patterns and size-based buckets.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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
        assert!(config.approvals.is_some());
        assert!(config.size.is_some());

        let approvals = config.approvals.unwrap();
        assert_eq!(approvals.len(), 2);
        assert_eq!(approvals[0].range, "0");
        assert_eq!(approvals[0].effect, "x0.5");
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
    }

    #[test]
    fn test_full_scoring_config_parse() {
        let yaml = r#"
base_score: 100
age: "+1 per 1h"
approvals:
  - range: "0"
    effect: "x0.5"
  - range: ">0"
    effect: "x2 per 1"
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

        let approvals = config.approvals.unwrap();
        assert_eq!(approvals.len(), 2);

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
}
