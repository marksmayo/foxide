use crate::model::{Run, percentile};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, fs, path::Path};

fn collect(path: &Path, runs: &mut Vec<Run>) -> Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            collect(&entry?.path(), runs)?;
        }
    } else if path.file_name().is_some_and(|n| n == "run.json") {
        let run: Run = serde_json::from_slice(&fs::read(path)?)
            .with_context(|| format!("Reading {}", path.display()))?;
        ensure!(
            run.schema_version == 1,
            "Unsupported run schema in {}",
            path.display()
        );
        runs.push(run);
    }
    Ok(())
}

fn cell(value: Option<f64>) -> String {
    value.map_or_else(|| "—".into(), |v| format!("{v:.2}"))
}
fn escape(value: &str) -> String {
    value.replace('|', "\\|").replace(['\n', '\r'], " ")
}

pub fn render(path: &Path) -> Result<String> {
    let mut runs = vec![];
    collect(path, &mut runs)?;
    ensure!(!runs.is_empty(), "No run.json files found");
    let mut groups: BTreeMap<String, Vec<&Run>> = BTreeMap::new();
    let mut incomplete = 0;
    for run in &runs {
        if !matches!(
            run.status.as_str(),
            "internal-restart-observed" | "process-sampled-playback-unverified"
        ) {
            incomplete += 1;
            continue;
        }
        let options: Vec<_> = run
            .args
            .iter()
            .filter(|a| !a.starts_with("--script="))
            .collect();
        // Do not silently merge cache states, hosts, fixtures, tuning, intervals or durations.
        let key = serde_json::to_string(&(
            &run.host,
            &run.os,
            &run.arch,
            &run.player,
            &run.profile,
            &run.media,
            &run.executable,
            &run.cache,
            run.duration_seconds,
            run.warmup_seconds,
            run.sample_interval_ms,
            options,
        ))?;
        groups.entry(key).or_default().push(run);
    }
    let mut out = format!(
        "# Foxide local process benchmark\n\nDisplay TTFF, watts, VRAM and presentation pacing are unmeasured. Restart is an internal proxy, including parent log polling delay. RSS/CPU do not validate playback. {incomplete} incomplete runs excluded; inspect their logs. Priming runs excluded.\n\n| Player/profile | Host | Media | Cache | n | Restart proxy p50/p95 ms | RSS peak MiB | Steady RSS mean MiB | Steady CPU mean % (one core) | Display TTFF | Package W | VRAM |\n|---|---|---|---|---:|---:|---:|---:|---:|---|---|---|\n"
    );
    for group in groups.values() {
        let r = group[0];
        let restart: Vec<_> = group
            .iter()
            .filter_map(|r| r.startup_restart_observed_ms)
            .collect();
        let peak = group
            .iter()
            .filter_map(|r| r.rss_peak_bytes)
            .max()
            .map(|v| v as f64 / 1048576.0);
        let rss: Vec<_> = group
            .iter()
            .filter_map(|r| r.rss_steady_mean_bytes)
            .collect();
        let cpu: Vec<_> = group
            .iter()
            .filter_map(|r| r.cpu_steady_mean_percent_one_core)
            .collect();
        let mean = |values: &[f64]| {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        };
        out.push_str(&format!(
            "| {} / {} | {} | {} | {} | {} | {} / {} | {} | {} | {} | — | — | — |\n",
            escape(&r.player),
            escape(&r.profile),
            escape(&r.host),
            escape(&r.media),
            escape(&r.cache),
            group.len(),
            cell(percentile(&restart, 0.5)),
            cell(percentile(&restart, 0.95)),
            cell(peak),
            cell(mean(&rss).map(|v| v / 1048576.0)),
            cell(mean(&cpu))
        ));
    }
    out.push_str("\nNearest-rank percentiles; small n is diagnostic only. Run metadata records the command, sample interval and duration. Report grouping uses fixture paths and executable paths; verify immutable media/tool manifests before comparing runs across sessions.\n");
    Ok(out)
}
