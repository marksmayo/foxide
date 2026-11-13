use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Sample {
    pub elapsed_ms: f64,
    pub rss_bytes: u64,
    /// 100% = one fully occupied logical CPU; may exceed 100%.
    pub cpu_percent_one_core: Option<f32>,
    pub process_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Run {
    pub schema_version: u32,
    pub player: String,
    pub profile: String,
    pub media: String,
    pub executable: String,
    pub args: Vec<String>,
    pub os: String,
    pub arch: String,
    pub host: String,
    pub cache: String,
    pub cache_evidence: Option<String>,
    pub pid: u32,
    pub started_unix_ms: u128,
    pub duration_seconds: f64,
    pub warmup_seconds: f64,
    pub sample_interval_ms: u64,
    pub exit_status: Option<String>,
    pub terminated_by_harness: bool,
    pub status: String,
    pub notes: Vec<String>,
    pub samples: Vec<Sample>,
    pub rss_peak_bytes: Option<u64>,
    pub rss_steady_mean_bytes: Option<f64>,
    pub cpu_steady_mean_percent_one_core: Option<f64>,
    /// Parent receipt of Lua playback-restart: includes polling and log flush delay.
    pub startup_restart_observed_ms: Option<f64>,
    /// Actual display TTFF stays null until a content-aware presentation collector exists.
    pub ttff_display_ms: Option<f64>,
    pub package_power_watts: Option<f64>,
    pub gpu_utilization_percent: Option<f64>,
    pub vram_bytes: Option<u64>,
    pub frame_pacing_stddev_ms: Option<f64>,
    pub instrumentation: Vec<serde_json::Value>,
}

pub fn percentile(values: &[f64], quantile: f64) -> Option<f64> {
    let mut sorted: Vec<_> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    let rank = (quantile.clamp(0.0, 1.0) * sorted.len() as f64).ceil() as usize;
    Some(sorted[rank.saturating_sub(1).min(sorted.len() - 1)])
}

pub fn summarize(run: &mut Run) {
    run.rss_peak_bytes = run.samples.iter().map(|s| s.rss_bytes).max();
    let steady: Vec<_> = run
        .samples
        .iter()
        .filter(|s| s.elapsed_ms >= run.warmup_seconds * 1000.0)
        .collect();
    if !steady.is_empty() {
        run.rss_steady_mean_bytes =
            Some(steady.iter().map(|s| s.rss_bytes as f64).sum::<f64>() / steady.len() as f64);
    }
    let cpu: Vec<_> = steady
        .iter()
        .filter_map(|s| s.cpu_percent_one_core.map(f64::from))
        .collect();
    if !cpu.is_empty() {
        run.cpu_steady_mean_percent_one_core = Some(cpu.iter().sum::<f64>() / cpu.len() as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_and_missing_data_are_explicit() {
        assert_eq!(percentile(&[], 0.95), None);
        assert_eq!(percentile(&[10.0, 1.0, 2.0, f64::NAN], 0.5), Some(2.0));
        assert_eq!(percentile(&[10.0, 1.0, 2.0], 0.95), Some(10.0));
    }
}
