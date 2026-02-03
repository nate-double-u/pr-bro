use anyhow::{bail, Result};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RangeOp {
    LessThan(u64),
    LessEqual(u64),
    GreaterThan(u64),
    GreaterEqual(u64),
    Equal(u64),
    Between(u64, u64), // Inclusive range: N-M
}

impl RangeOp {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        if let Some(val) = s.strip_prefix(">=") {
            Ok(RangeOp::GreaterEqual(val.trim().parse()?))
        } else if let Some(val) = s.strip_prefix("<=") {
            Ok(RangeOp::LessEqual(val.trim().parse()?))
        } else if let Some(val) = s.strip_prefix(">") {
            Ok(RangeOp::GreaterThan(val.trim().parse()?))
        } else if let Some(val) = s.strip_prefix("<") {
            Ok(RangeOp::LessThan(val.trim().parse()?))
        } else if s.contains('-') && !s.starts_with('-') {
            // Range format: "100-500"
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() == 2 {
                let low: u64 = parts[0].trim().parse()?;
                let high: u64 = parts[1].trim().parse()?;
                Ok(RangeOp::Between(low, high))
            } else {
                bail!("Invalid range format: {}", s)
            }
        } else {
            Ok(RangeOp::Equal(s.parse()?))
        }
    }

    pub fn matches(&self, value: u64) -> bool {
        match self {
            RangeOp::LessThan(n) => value < *n,
            RangeOp::LessEqual(n) => value <= *n,
            RangeOp::GreaterThan(n) => value > *n,
            RangeOp::GreaterEqual(n) => value >= *n,
            RangeOp::Equal(n) => value == *n,
            RangeOp::Between(low, high) => value >= *low && value <= *high,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    Add(f64),
    Multiply(f64),
    AddPerUnit(f64, Duration),
    MultiplyPerUnit(f64, Duration),
}

impl Effect {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        // Check for "per" modifier
        if let Some((effect_part, per_part)) = s.split_once(" per ") {
            let duration = humantime::parse_duration(per_part.trim())?;
            if let Some(val) = effect_part.strip_prefix('+') {
                Ok(Effect::AddPerUnit(val.trim().parse()?, duration))
            } else if let Some(val) = effect_part.strip_prefix('x') {
                Ok(Effect::MultiplyPerUnit(val.trim().parse()?, duration))
            } else {
                bail!("Effect must start with + or x: {}", s)
            }
        } else if let Some(val) = s.strip_prefix('+') {
            Ok(Effect::Add(val.trim().parse()?))
        } else if let Some(val) = s.strip_prefix('x') {
            Ok(Effect::Multiply(val.trim().parse()?))
        } else {
            bail!("Effect must start with + or x: {}", s)
        }
    }

    /// Apply effect to score. `units` is number of time periods for per-unit effects.
    pub fn apply(&self, score: f64, units: u64) -> f64 {
        match self {
            Effect::Add(n) => score + n,
            Effect::Multiply(n) => score * n,
            Effect::AddPerUnit(n, _) => score + (n * units as f64),
            Effect::MultiplyPerUnit(n, _) => score * n.powf(units as f64),
        }
    }

    /// Get the duration for per-unit effects (for calculating units from PR age)
    pub fn unit_duration(&self) -> Option<Duration> {
        match self {
            Effect::AddPerUnit(_, d) | Effect::MultiplyPerUnit(_, d) => Some(*d),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_less_than() {
        let range = RangeOp::parse("<100").unwrap();
        assert!(range.matches(50));
        assert!(!range.matches(100));
        assert!(!range.matches(150));
    }

    #[test]
    fn test_parse_range_less_equal() {
        let range = RangeOp::parse("<=100").unwrap();
        assert!(range.matches(50));
        assert!(range.matches(100));
        assert!(!range.matches(101));
    }

    #[test]
    fn test_parse_range_greater_than() {
        let range = RangeOp::parse(">100").unwrap();
        assert!(!range.matches(50));
        assert!(!range.matches(100));
        assert!(range.matches(150));
    }

    #[test]
    fn test_parse_range_greater_equal() {
        let range = RangeOp::parse(">=100").unwrap();
        assert!(!range.matches(50));
        assert!(range.matches(100));
        assert!(range.matches(150));
    }

    #[test]
    fn test_parse_range_equal() {
        let range = RangeOp::parse("0").unwrap();
        assert!(range.matches(0));
        assert!(!range.matches(1));
    }

    #[test]
    fn test_parse_range_between() {
        let range = RangeOp::parse("100-500").unwrap();
        assert!(!range.matches(50));
        assert!(range.matches(100));
        assert!(range.matches(300));
        assert!(range.matches(500));
        assert!(!range.matches(501));
    }

    #[test]
    fn test_parse_effect_add() {
        let effect = Effect::parse("+10").unwrap();
        assert_eq!(effect.apply(100.0, 1), 110.0);
    }

    #[test]
    fn test_parse_effect_multiply() {
        let effect = Effect::parse("x2").unwrap();
        assert_eq!(effect.apply(100.0, 1), 200.0);
    }

    #[test]
    fn test_parse_effect_add_per_unit() {
        // "+1 per 1h" on 5 hours = +5
        let effect = Effect::parse("+1 per 1h").unwrap();
        assert_eq!(effect.apply(100.0, 5), 105.0); // 5 units
    }

    #[test]
    fn test_parse_effect_multiply_per_unit() {
        // "x1.1 per 1h" on 3 hours = x1.1^3
        let effect = Effect::parse("x1.1 per 1h").unwrap();
        let result = effect.apply(100.0, 3); // 3 units
        assert!((result - 133.1).abs() < 0.1); // 100 * 1.1^3
    }

    #[test]
    fn test_parse_effect_negative_add() {
        let effect = Effect::parse("+-5").unwrap();
        assert_eq!(effect.apply(100.0, 1), 95.0);
    }

    #[test]
    fn test_parse_effect_decimal_multiply() {
        let effect = Effect::parse("x0.5").unwrap();
        assert_eq!(effect.apply(100.0, 1), 50.0);
    }
}
