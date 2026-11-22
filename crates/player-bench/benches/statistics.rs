//! CPU-only benchmarks of the production telemetry model and its JSON schema.
use divan::{Bencher, black_box};
use player_bench::model::{Run, Sample, percentile, summarize};

fn main() {
    divan::main();
}

fn run_fixture(count: usize, cpu_mode: u8) -> Run {
    Run {
        schema_version: 1,
        player: "foxide".into(),
        profile: "avc1080".into(),
        media: "/media/Series/Épisode 01.mkv".into(),
        executable: "/opt/foxide".into(),
        args: vec!["--headless".into(), "--speed=1".into()],
        os: "benchmark-host".into(),
        arch: "benchmark-arch".into(),
        host: "fixed-fixture".into(),
        cache: "uncontrolled".into(),
        cache_evidence: None,
        pid: 1,
        started_unix_ms: 0,
        sampling_started_ms: 0.0,
        duration_seconds: count as f64 * 0.25,
        warmup_seconds: 2.0,
        sample_interval_ms: 250,
        exit_status: None,
        terminated_by_harness: true,
        status: "internal-restart-observed".into(),
        notes: vec!["Synthetic telemetry; not a player performance measurement".into()],
        samples: (0..count)
            .map(|i| Sample {
                elapsed_ms: i as f64 * 250.0,
                rss_bytes: 80 * 1024 * 1024 + (i % 128) as u64 * 4096,
                cpu_percent_one_core: if cpu_mode == 0 || (cpu_mode == 2 && i % 4 == 0) {
                    None
                } else {
                    Some((i * 17 % 400) as f32 / 4.0)
                },
                process_count: 2,
            })
            .collect(),
        rss_peak_bytes: None,
        rss_steady_mean_bytes: None,
        cpu_steady_mean_percent_one_core: None,
        startup_restart_observed_ms: Some(100.0),
        ttff_display_ms: None,
        package_power_watts: None,
        gpu_utilization_percent: None,
        vram_bytes: None,
        frame_pacing_stddev_ms: None,
        instrumentation: vec![],
    }
}

#[divan::bench(consts = [50, 95, 99], args = [0, 1, 16, 256, 4096, 65536])]
fn quantile<const Q: u8>(bencher: Bencher, count: usize) {
    let values: Vec<_> = (0..count).map(|i| (i * 7919 % 65521) as f64).collect();
    let q = f64::from(Q) / 100.0;
    assert_eq!(percentile(&values, q).is_some(), count > 0);
    bencher.bench_local(|| percentile(black_box(&values), black_box(q)));
}

#[divan::bench(args = ["ascending", "descending", "duplicates", "nonfinite", "spikes"])]
fn percentile_distribution(bencher: Bencher, distribution: &str) {
    let values: Vec<_> = (0..4096)
        .map(|i| match distribution {
            "ascending" => i as f64,
            "descending" => (4096 - i) as f64,
            "duplicates" => (i % 4) as f64,
            "nonfinite" if i % 3 == 0 => f64::NAN,
            "nonfinite" if i % 3 == 1 => f64::INFINITY,
            "nonfinite" => i as f64,
            "spikes" => {
                if i % 100 == 0 {
                    10000.0
                } else {
                    1.0
                }
            }
            _ => unreachable!(),
        })
        .collect();
    assert!(percentile(&values, 0.95).unwrap().is_finite());
    bencher.bench_local(|| percentile(black_box(&values), black_box(0.95)));
}

#[divan::bench(consts = [0, 1, 2], args = [0, 1, 40, 2400, 40000])]
fn summarize_cpu_availability<const CPU: u8>(bencher: Bencher, count: usize) {
    let mut check = run_fixture(count, CPU);
    summarize(&mut check);
    assert_eq!(check.rss_peak_bytes.is_some(), count > 0);
    if CPU == 0 {
        assert!(check.cpu_steady_mean_percent_one_core.is_none());
    }
    bencher
        .with_inputs(|| run_fixture(count, CPU))
        .bench_local_refs(|run| {
            summarize(black_box(&mut *run));
            black_box((
                run.rss_peak_bytes,
                run.rss_steady_mean_bytes,
                run.cpu_steady_mean_percent_one_core,
            ))
        });
}

#[divan::bench(consts = [0, 50, 100], args = [40, 2400, 40000])]
fn summarize_warmup<const WARMUP: u8>(bencher: Bencher, count: usize) {
    let make = || {
        let mut run = run_fixture(count, 1);
        run.warmup_seconds = run.duration_seconds * f64::from(WARMUP) / 100.0;
        run
    };
    let mut check = make();
    summarize(&mut check);
    assert_eq!(check.rss_steady_mean_bytes.is_some(), WARMUP < 100);
    bencher.with_inputs(make).bench_local_refs(|run| {
        summarize(black_box(&mut *run));
        black_box((
            run.rss_peak_bytes,
            run.rss_steady_mean_bytes,
            run.cpu_steady_mean_percent_one_core,
        ))
    });
}

#[divan::bench(args = [0, 40, 2400])]
fn serialize_run(bencher: Bencher, count: usize) {
    let run = run_fixture(count, 2);
    let encoded = serde_json::to_vec(&run).unwrap();
    assert_eq!(
        serde_json::from_slice::<Run>(&encoded)
            .unwrap()
            .samples
            .len(),
        count
    );
    bencher.bench_local(|| serde_json::to_vec(black_box(&run)).unwrap());
}

#[divan::bench(args = [0, 40, 2400])]
fn deserialize_run(bencher: Bencher, count: usize) {
    let encoded = serde_json::to_vec(&run_fixture(count, 2)).unwrap();
    assert_eq!(
        serde_json::from_slice::<Run>(&encoded)
            .unwrap()
            .samples
            .len(),
        count
    );
    bencher.bench_local(|| serde_json::from_slice::<Run>(black_box(&encoded)).unwrap());
}

#[divan::bench(args = [0, 40, 2400])]
fn summarize_and_serialize(bencher: Bencher, count: usize) {
    bencher
        .with_inputs(|| run_fixture(count, 2))
        .bench_local_refs(|run| {
            summarize(black_box(&mut *run));
            serde_json::to_vec(black_box(run)).unwrap()
        });
}
