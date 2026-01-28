use crate::github::types::PullRequest;
use super::config::ScoringConfig;
use super::factors::Effect;

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub score: f64,
    pub incomplete: bool,
}

pub fn calculate_score(pr: &PullRequest, config: &ScoringConfig) -> ScoreResult {
    let mut score = config.base_score.unwrap_or(100.0);
    let incomplete = false;

    // Apply age factor (always available - created_at always present)
    if let Some(ref age_str) = config.age {
        if let Ok(effect) = Effect::parse(age_str) {
            let age = pr.age();
            let units = calculate_units(&effect, age);
            score = effect.apply(score, units);
        }
    }

    // Apply approvals factor
    if let Some(ref buckets) = config.approvals {
        score = apply_bucket_effect(score, pr.approvals as u64, buckets, |b| &b.range, |b| &b.effect);
    }

    // Apply size factor
    if let Some(ref size_config) = config.size {
        let size = pr.size();
        score = apply_bucket_effect(score, size, &size_config.buckets, |b| &b.range, |b| &b.effect);
    }

    // Floor at zero
    ScoreResult {
        score: score.max(0.0),
        incomplete,
    }
}

fn calculate_units(effect: &Effect, age: chrono::Duration) -> u64 {
    if let Some(unit_duration) = effect.unit_duration() {
        let age_secs = age.num_seconds().max(0) as u64;
        let unit_secs = unit_duration.as_secs();
        if unit_secs > 0 {
            age_secs / unit_secs
        } else {
            0
        }
    } else {
        1  // Non-per-unit effects apply once
    }
}

fn apply_bucket_effect<T, F1, F2>(score: f64, value: u64, buckets: &[T], get_range: F1, get_effect: F2) -> f64
where
    F1: Fn(&T) -> &str,
    F2: Fn(&T) -> &str,
{
    use super::factors::RangeOp;

    for bucket in buckets {
        if let Ok(range) = RangeOp::parse(get_range(bucket)) {
            if range.matches(value) {
                if let Ok(effect) = Effect::parse(get_effect(bucket)) {
                    return effect.apply(score, 1);
                }
            }
        }
    }
    score  // No matching bucket, return unchanged
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration as ChronoDuration};
    use crate::scoring::{ApprovalBucket, SizeBucket, SizeConfig};

    fn sample_pr(age_hours: i64, approvals: u32, size: u64) -> PullRequest {
        PullRequest {
            title: "Test PR".to_string(),
            number: 1,
            author: "user".to_string(),
            repo: "owner/repo".to_string(),
            url: "https://github.com/owner/repo/pull/1".to_string(),
            created_at: Utc::now() - ChronoDuration::hours(age_hours),
            updated_at: Utc::now(),
            additions: size / 2,
            deletions: size / 2,
            approvals,
            draft: false,
        }
    }

    #[test]
    fn test_base_score_only() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(&pr, &ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
        });
        assert_eq!(result.score, 100.0);
        assert!(!result.incomplete);
    }

    #[test]
    fn test_age_factor_additive() {
        let pr = sample_pr(5, 0, 100);
        let result = calculate_score(&pr, &ScoringConfig {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),
            approvals: None,
            size: None,
        });
        assert_eq!(result.score, 105.0);  // 100 + 5*1
    }

    #[test]
    fn test_score_floors_at_zero() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(&pr, &ScoringConfig {
            base_score: Some(10.0),
            age: Some("+-20 per 1h".to_string()),  // Would go negative
            approvals: None,
            size: None,
        });
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn test_approvals_bucket_zero() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(&pr, &ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: Some(vec![
                ApprovalBucket { range: "0".to_string(), effect: "x0.5".to_string() },
            ]),
            size: None,
        });
        assert_eq!(result.score, 50.0);
    }

    #[test]
    fn test_size_bucket() {
        let pr = sample_pr(1, 0, 50);
        let result = calculate_score(&pr, &ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: Some(SizeConfig {
                exclude: None,
                buckets: vec![
                    SizeBucket { range: "<100".to_string(), effect: "x2".to_string() },
                ],
            }),
        });
        assert_eq!(result.score, 200.0);
    }

    #[test]
    fn test_full_scoring_flow() {
        // PR: 24h old, 1 approval, 150 lines
        let pr = sample_pr(24, 1, 150);

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),  // +24 for age
            approvals: Some(vec![
                ApprovalBucket { range: "0".to_string(), effect: "x0.5".to_string() },
                ApprovalBucket { range: ">0".to_string(), effect: "x1.5".to_string() },
            ]),
            size: Some(SizeConfig {
                exclude: None,
                buckets: vec![
                    SizeBucket { range: "<100".to_string(), effect: "x2".to_string() },
                    SizeBucket { range: ">=100".to_string(), effect: "x1".to_string() },
                ],
            }),
        };

        let result = calculate_score(&pr, &config);

        // Expected: (100 + 24) * 1.5 * 1 = 186
        assert!((result.score - 186.0).abs() < 0.1);
        assert!(!result.incomplete);
    }

    #[test]
    fn test_default_config_scoring() {
        let pr = sample_pr(5, 0, 50);
        let config = ScoringConfig::default();
        let result = calculate_score(&pr, &config);

        // Default config has factors: base=100, +1/h age, 0 approvals=x0.5, <100 size=x5
        // Expected: (100 + 5) * 0.5 * 5 = 262.5
        assert!((result.score - 262.5).abs() < 0.1);
    }

    #[test]
    fn test_multiplicative_age_factor() {
        let pr = sample_pr(3, 0, 100);
        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("x1.1 per 1h".to_string()),
            approvals: None,
            size: None,
        };

        let result = calculate_score(&pr, &config);
        // 100 * 1.1^3 = 133.1
        assert!((result.score - 133.1).abs() < 0.1);
    }

    #[test]
    fn test_bucket_first_match_wins() {
        let pr = sample_pr(1, 0, 50);  // Size 50

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: Some(SizeConfig {
                exclude: None,
                buckets: vec![
                    SizeBucket { range: "<100".to_string(), effect: "x2".to_string() },  // Matches first
                    SizeBucket { range: "<200".to_string(), effect: "x3".to_string() },  // Also matches but not used
                ],
            }),
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 200.0);  // First match (x2), not second (x3)
    }
}
