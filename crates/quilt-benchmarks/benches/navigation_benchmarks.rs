//! Navigation performance benchmarks
//!
//! Target: P95 < 200ms for route matching

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

pub fn navigation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("navigation");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(20);

    // Simulate route matching for journal
    group.bench_function("route_match_journal", |b| {
        b.iter(|| {
            let path = "/journal";
            // Simulate route matching logic
            black_box(path.starts_with("/journal") || path == "/");
        });
    });

    // Simulate route matching for page
    group.bench_function("route_match_page", |b| {
        b.iter(|| {
            let path = "/page/abc-def-123";
            // Simulate route matching with UUID pattern
            black_box(path.starts_with("/page/") && path.len() > 6);
        });
    });

    // Simulate route matching for block
    group.bench_function("route_match_block", |b| {
        b.iter(|| {
            let path = "/block/xyz-789";
            // Simulate route matching with block ID
            black_box(path.starts_with("/block/") && path.len() > 7);
        });
    });

    // Simulate route matching for namespace
    group.bench_function("route_match_namespace", |b| {
        b.iter(|| {
            let path = "/namespace/project/tasks";
            // Simulate nested namespace route matching
            black_box(path.starts_with("/namespace/"));
        });
    });

    // Simulate full route resolution with multiple patterns
    group.bench_function("route_resolve_full", |b| {
        b.iter(|| {
            let path = "/journal/2024/05/21";
            // Simulate full route resolution
            let result = if path.starts_with("/journal/") {
                Some(("journal", &path[9..]))
            } else if path.starts_with("/page/") {
                Some(("page", &path[6..]))
            } else if path.starts_with("/block/") {
                Some(("block", &path[7..]))
            } else if path == "/" {
                Some(("home", ""))
            } else {
                None
            };
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(benches, navigation_benchmarks);
criterion_main!(benches);
