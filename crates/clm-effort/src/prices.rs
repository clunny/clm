//! What a usage costs at list prices (`effort/prices`): an estimate, never a bill.

use clm_ledger::Usage;

/// USD per million tokens: input, output, cache reads, cache writes (the 5-minute rate).
type Rates = [f64; 4];

/// By a pattern the lowercased model ID contains, most specific first, as checked against
/// the list prices on 2026-10-30. Haiku 5.5 at its rate for prompts of 100K tokens or fewer.
const PRICES: [(&str, Rates); 14] = [
    ("fable-5-1", [10.0, 50.0, 0.25, 12.5]),
    ("mythos-5-1", [10.0, 50.0, 0.25, 12.5]),
    ("fable", [10.0, 50.0, 1.0, 12.5]),
    ("mythos", [10.0, 50.0, 1.0, 12.5]),
    ("opus-5-5", [4.0, 20.0, 0.2, 5.0]),
    ("opus-5", [5.0, 25.0, 0.5, 6.25]),
    ("opus-4-5", [5.0, 25.0, 0.5, 6.25]),
    ("opus-4-6", [5.0, 25.0, 0.5, 6.25]),
    ("opus-4-7", [5.0, 25.0, 0.5, 6.25]),
    ("opus-4-8", [5.0, 25.0, 0.5, 6.25]),
    ("opus", [15.0, 75.0, 1.5, 18.75]),
    ("sonnet-5", [2.0, 10.0, 0.2, 2.5]),
    ("haiku-5", [0.1, 0.5, 0.01, 0.125]),
    ("haiku", [1.0, 5.0, 0.1, 1.25]),
];

/// For a model no row matches.
const SONNET: Rates = [3.0, 15.0, 0.3, 3.75];

/// The estimated cost in USD of `usage` on `model`.
pub fn dollars(model: &str, usage: &Usage) -> f64 {
    let model = model.to_lowercase();
    let [i, o, r, w] = PRICES
        .iter()
        .find(|(pattern, _)| model.contains(pattern))
        .map_or(SONNET, |(_, rates)| *rates);
    (usage.input as f64 * i
        + usage.output as f64 * o
        + usage.cache_read as f64 * r
        + usage.cache_write as f64 * w)
        / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_model_uses_fallback() {
        // req:effort/prices@b1a2b29a
        let million = Usage {
            input: 1_000_000,
            ..Usage::default()
        };
        assert_eq!(dollars("some-new-model", &million), 3.0);
        assert_eq!(dollars("claude-sonnet-4-6", &million), 3.0);
        assert_eq!(dollars("claude-opus-5-5", &million), 4.0);
        assert_eq!(dollars("claude-opus-5", &million), 5.0);
        assert_eq!(dollars("Claude-Fable-5-1", &million), 10.0);
        let reads = Usage {
            cache_read: 1_000_000,
            ..Usage::default()
        };
        assert_eq!(dollars("claude-fable-5-1", &reads), 0.25);
        assert_eq!(dollars("claude-fable-5", &reads), 1.0);
        let all = Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 1_000_000,
            cache_write: 1_000_000,
        };
        assert_eq!(dollars("claude-haiku-4-5", &all), 1.0 + 5.0 + 0.1 + 1.25);
    }
}
