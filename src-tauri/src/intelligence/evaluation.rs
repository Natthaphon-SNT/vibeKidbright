//! Reproducible file-level qrels; model results are never replaced with mock scores.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Debug, Deserialize)]
pub struct QueryCase {
    pub id: String,
    pub query: String,
    pub language: String,
    pub relevant_files: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Metrics {
    pub recall_at_5: f64,
    pub precision_at_5: f64,
    pub mrr: f64,
    pub ndcg_at_5: f64,
}
pub fn metrics(ranked: &[String], relevant: &[String]) -> Metrics {
    let expected: HashSet<_> = relevant.iter().collect();
    let mut seen = HashSet::new();
    let ranked: Vec<_> = ranked
        .iter()
        .filter(|file| seen.insert(*file))
        .take(5)
        .collect();
    let found = ranked
        .iter()
        .filter(|file| expected.contains(*file))
        .count();
    let dcg: f64 = ranked
        .iter()
        .enumerate()
        .filter(|(_, file)| expected.contains(*file))
        .map(|(rank, _)| 1.0 / ((rank + 2) as f64).log2())
        .sum();
    let ideal: f64 = (0..expected.len().min(5))
        .map(|rank| 1.0 / ((rank + 2) as f64).log2())
        .sum();
    Metrics {
        recall_at_5: found as f64 / expected.len().max(1) as f64,
        precision_at_5: found as f64 / 5.,
        mrr: ranked
            .iter()
            .position(|file| expected.contains(file))
            .map(|r| 1. / (r + 1) as f64)
            .unwrap_or(0.),
        ndcg_at_5: if ideal > 0. { dcg / ideal } else { 0. },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_chunks_do_not_inflate_document_recall() {
        let score = metrics(
            &["b".into(), "b".into(), "a".into()],
            &["a".into(), "b".into()],
        );
        assert_eq!(score.recall_at_5, 1.);
        assert_eq!(score.precision_at_5, 0.4);
        assert_eq!(score.mrr, 1.);
        assert!((score.ndcg_at_5 - 1.).abs() < 1e-10);
    }
}
