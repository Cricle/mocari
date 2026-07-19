//! Compatibility tests for `motion3.json` parsing and curve sampling against
//! the official Cubism SDK 4 motion format.
//!
//! These tests ingest every `motion3.json` shipped under `assets/models/...`
//! and assert that mocari's `Motion3` parser produces results that are
//! internally consistent and stable: every curve samples to a finite value at
//! every authoring-FPS tick between 0 and duration, monotonic time advances
//! across segments, and the overall fingerprint (sum of sampled values per
//! curve) is stable across runs. The fingerprint is also compared against a
//! baseline written into a fixture TSV, so any change in sampling math will
//! surface here as a regression.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use mocari::json::Motion3;

/// Aggregates a single (curve_target, curve_id) fingerprint across all
/// sampled ticks. Stored as 32-bit hash so deterministic across runs.
fn curve_fingerprint(motion: &Motion3, fps: f32) -> BTreeMap<(String, String), u64> {
    let duration = motion.meta().duration();
    let mut fingerprints: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut tick = 0.0f32;
    let step = if fps > 0.0 { 1.0 / fps } else { 1.0 / 30.0 };
    while tick <= duration + 1e-6 {
        for curve in motion.curves() {
            if let Some(value) = curve.sample(tick) {
                assert!(
                    value.is_finite(),
                    "non-finite value at t={} on {} / {}: {}",
                    tick,
                    curve.target(),
                    curve.id(),
                    value,
                );
                let entry = fingerprints
                    .entry((curve.target().to_string(), curve.id().to_string()))
                    .or_insert(0u64);
                *entry = entry.wrapping_add(value.to_bits() as u64).wrapping_add(1);
            }
        }
        tick += step;
    }
    fingerprints
}

fn collect_motion_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let motions = entry.path().join("motions");
            if let Ok(motion_entries) = fs::read_dir(&motions) {
                for m in motion_entries.flatten() {
                    let p = m.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("json")
                        && p.to_string_lossy().ends_with(".motion3.json")
                    {
                        paths.push(p);
                    }
                }
            }
        }
    }
    paths.sort();
    paths
}

#[test]
fn every_motion3_samples_finite_at_each_tick() {
    for path in collect_motion_files() {
        let source = fs::read_to_string(&path).expect("read motion3");
        let motion = Motion3::from_json_str(&source).unwrap_or_else(|err| {
            panic!(
                "mocari failed to parse official motion3 {}: {:?}",
                path.display(),
                err
            );
        });

        // Every curve must have a finite, sorted progression of segment times.
        for curve in motion.curves() {
            let first = curve.first_point();
            let mut prev_time = first.time;
            for segment in curve.segments() {
                let end = segment.end();
                assert!(
                    end.time >= prev_time,
                    "non-monotonic segment time in {}: prev={}, end={} on {}/{}",
                    path.display(),
                    prev_time,
                    end.time,
                    curve.target(),
                    curve.id(),
                );
                prev_time = end.time;
            }
        }

        // Sample fingerprints must be stable (non-panicking, all finite).
        let fps = motion.meta().fps();
        let _prints = curve_fingerprint(&motion, fps);
    }
}

#[test]
fn motion3_fingerprints_match_baseline() {
    let baseline_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compat/fixtures/motion3_baseline.tsv");
    fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();
    let baseline = if baseline_path.exists() {
        fs::read_to_string(&baseline_path).unwrap()
    } else {
        String::new()
    };

    let mut actual = String::new();
    for path in collect_motion_files() {
        let source = fs::read_to_string(&path).unwrap();
        let motion = Motion3::from_json_str(&source).unwrap();
        let fps = motion.meta().fps();
        for ((target, id), fp) in curve_fingerprint(&motion, fps) {
            actual.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                path.file_name().unwrap().to_string_lossy(),
                target,
                id,
                fp,
            ));
        }
    }

    if baseline.is_empty() {
        fs::write(&baseline_path, &actual).unwrap();
        eprintln!(
            "wrote baseline to {} (first-time fixture; subsequent runs will check against it)",
            baseline_path.display(),
        );
        return;
    }

    if baseline != actual {
        let diff_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compat/fixtures/motion3_baseline.tsv.diff");
        fs::write(&diff_path, &actual).unwrap();
        panic!(
            "motion3 sampling fingerprint drifted; baseline:\n  {}\nnew values written to:\n  {}\nRe-generate the baseline by removing the .tsv file if this drift is intended.",
            baseline_path.display(),
            diff_path.display(),
        );
    }
}
