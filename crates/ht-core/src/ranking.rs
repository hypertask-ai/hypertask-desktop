use crate::Result;
use anyhow::{Context, ensure};

/// Web generateRank's numeric rank format. A midpoint avoids random near-collisions.
pub fn task_rank(previous: Option<&str>, next: Option<&str>) -> Result<String> {
    if previous.is_none() && next.is_none() {
        return Ok("A0100".into());
    }
    let number = |rank: &str| -> Result<f64> {
        let value: f64 = rank
            .trim_start_matches(|c: char| c.is_ascii_alphabetic())
            .parse()
            .context("invalid task ranking")?;
        ensure!(value.is_finite(), "invalid task ranking");
        Ok(value)
    };
    let value = match (previous, next) {
        (None, Some(next)) => number(next)? / 2.,
        (Some(previous), None) => number(previous)? + 90.,
        (Some(previous), Some(next)) => {
            let (a, b) = (number(previous)?, number(next)?);
            ensure!(a < b, "task ranks need refresh");
            (a + b) / 2.
        }
        _ => unreachable!(),
    };
    let source = previous.or(next).unwrap();
    let prefix: String = source
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let rank = format!("{prefix}{:0>20}", format!("{value:.15}"));
    ensure!(
        previous.is_none_or(|p| rank.as_str() > p) && next.is_none_or(|n| rank.as_str() < n),
        "task ranks need refresh"
    );
    Ok(rank)
}
