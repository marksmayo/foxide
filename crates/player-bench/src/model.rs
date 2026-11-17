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
    /// Sampling begins this many milliseconds after process spawn (zero in legacy records).
    #[serde(default)]
    pub sampling_started_ms: f64,
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
    if values.is_empty() {
        return None;
    }
    let mut sorted: Vec<_> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    let rank = (quantile.clamp(0.0, 1.0) * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(*sorted.select_nth_unstable_by(index, f64::total_cmp).1)
}

pub fn summarize(run: &mut Run) {
    run.rss_steady_mean_bytes = None;
    run.cpu_steady_mean_percent_one_core = None;
    if run.samples.is_empty() {
        run.rss_peak_bytes = None;
        return;
    }
    run.rss_peak_bytes = run.samples.iter().map(|s| s.rss_bytes).max();
    let mut rss_sum = 0.0;
    let mut rss_count = 0;
    let mut cpu_sum = 0.0;
    let mut cpu_count = 0;
    let warmup_end_ms = run.sampling_started_ms + run.warmup_seconds * 1000.0;
    for sample in &run.samples {
        if sample.elapsed_ms >= warmup_end_ms {
            rss_sum += sample.rss_bytes as f64;
            rss_count += 1;
            if let Some(cpu) = sample.cpu_percent_one_core {
                cpu_sum += f64::from(cpu);
                cpu_count += 1;
            }
        }
    }
    if rss_count > 0 {
        run.rss_steady_mean_bytes = Some(rss_sum / rss_count as f64);
    }
    if cpu_count > 0 {
        run.cpu_steady_mean_percent_one_core = Some(cpu_sum / cpu_count as f64);
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
