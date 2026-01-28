use anyhow::Result;
use super::config::ScoringConfig;
use super::factors::{RangeOp, Effect};

/// Validate scoring configuration at startup.
/// Returns all validation errors at once (not just the first).
pub fn validate_scoring(config: &ScoringConfig) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Validate base_score
    if let Some(base) = config.base_score {
        if base < 0.0 {
            errors.push("scoring.base_score: must be non-negative".to_string());
        }
    }

    // Validate age factor syntax
    if let Some(ref age) = config.age {
        if let Err(e) = Effect::parse(age) {
            errors.push(format!("scoring.age: invalid format '{}' - {}", age, e));
        }
    }

    // Validate approvals buckets
    if let Some(ref buckets) = config.approvals {
        for (i, bucket) in buckets.iter().enumerate() {
            if let Err(e) = RangeOp::parse(&bucket.range) {
                errors.push(format!(
                    "scoring.approvals[{}].range: invalid '{}' - {}",
                    i, bucket.range, e
                ));
            }
            if let Err(e) = Effect::parse(&bucket.effect) {
                errors.push(format!(
                    "scoring.approvals[{}].effect: invalid '{}' - {}",
                    i, bucket.effect, e
                ));
            }
        }
    }

    // Validate size buckets
    if let Some(ref size_config) = config.size {
        for (i, bucket) in size_config.buckets.iter().enumerate() {
            if let Err(e) = RangeOp::parse(&bucket.range) {
                errors.push(format!(
                    "scoring.size.buckets[{}].range: invalid '{}' - {}",
                    i, bucket.range, e
                ));
            }
            if let Err(e) = Effect::parse(&bucket.effect) {
                errors.push(format!(
                    "scoring.size.buckets[{}].effect: invalid '{}' - {}",
                    i, bucket.effect, e
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::{ApprovalBucket, SizeBucket, SizeConfig};

    #[test]
    fn test_valid_config() {
        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),
            approvals: Some(vec![
                ApprovalBucket { range: "0".to_string(), effect: "x0.5".to_string() },
            ]),
            size: None,
        };
        assert!(validate_scoring(&config).is_ok());
    }

    #[test]
    fn test_empty_config() {
        let config = ScoringConfig {
            base_score: None,
            age: None,
            approvals: None,
            size: None,
        };
        assert!(validate_scoring(&config).is_ok());
    }

    #[test]
    fn test_invalid_age_format() {
        let config = ScoringConfig {
            base_score: None,
            age: Some("invalid".to_string()),
            approvals: None,
            size: None,
        };
        let result = validate_scoring(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("scoring.age"));
    }

    #[test]
    fn test_negative_base_score() {
        let config = ScoringConfig {
            base_score: Some(-10.0),
            age: None,
            approvals: None,
            size: None,
        };
        let result = validate_scoring(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("base_score"));
    }

    #[test]
    fn test_invalid_approval_bucket() {
        let config = ScoringConfig {
            base_score: None,
            age: None,
            approvals: Some(vec![
                ApprovalBucket { range: "invalid".to_string(), effect: "x2".to_string() },
            ]),
            size: None,
        };
        let result = validate_scoring(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("scoring.approvals[0].range"));
    }

    #[test]
    fn test_invalid_size_bucket() {
        let config = ScoringConfig {
            base_score: None,
            age: None,
            approvals: None,
            size: Some(SizeConfig {
                exclude: None,
                buckets: vec![
                    SizeBucket { range: "<100".to_string(), effect: "bad".to_string() },
                ],
            }),
        };
        let result = validate_scoring(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("scoring.size.buckets[0].effect"));
    }

    #[test]
    fn test_collects_all_errors() {
        let config = ScoringConfig {
            base_score: Some(-10.0),  // Error 1
            age: Some("bad".to_string()),  // Error 2
            approvals: None,
            size: None,
        };
        let result = validate_scoring(&config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }
}
