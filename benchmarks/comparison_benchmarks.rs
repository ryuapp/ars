#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout
)]

/// Comparison benchmarks: ars vs url crate vs ada-url
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::process::Command;

// ars
use ars;

// Rust url crate
use url::Url as UrlCrate;

// ada-url
use ada_url::Url as AdaUrl;

/// Download dataset file before running benchmarks
fn ensure_dataset(path: &str, url: &str, name: &str) {
    // Download if file doesn't exist or is older than 24 hours
    let is_fresh = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| t.elapsed().ok().ok_or(std::io::Error::other("elapsed")))
        .is_ok_and(|elapsed| elapsed.as_secs() <= 86400);

    let should_download = !is_fresh;

    if should_download {
        println!("📥 Downloading latest {}...", name);

        let output = Command::new("curl")
            .args(&["-fsSL", "-o", path, url])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                println!("✓ Downloaded {} successfully", name);
            }
            _ => {
                println!("⚠ Failed to download, using existing file if available");
            }
        }
    } else {
        println!("✓ Using cached {}", name);
    }
}

/// Download top100.txt before running benchmarks
/// Dataset: https://github.com/ada-url/url-various-datasets
fn ensure_top100_txt() {
    ensure_dataset(
        "./benchmarks/url-various-datasets/top100.txt",
        "https://raw.githubusercontent.com/ada-url/url-various-datasets/main/top100/top100.txt",
        "top100.txt",
    );
}

/// Download wikipedia_100k.txt before running benchmarks
/// Dataset: https://github.com/ada-url/url-various-datasets
fn ensure_wikipedia_100k_txt() {
    ensure_dataset(
        "./benchmarks/url-various-datasets/wikipedia_100k.txt",
        "https://raw.githubusercontent.com/ada-url/url-various-datasets/main/wikipedia/wikipedia_100k.txt",
        "wikipedia_100k.txt",
    );
}

fn bench_parse_simple_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_simple");
    let input = "http://example.com/";

    group.bench_function("ars", |b| {
        b.iter(|| ars::Url::parse(black_box(input), None).unwrap());
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| UrlCrate::parse(black_box(input)).unwrap());
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| AdaUrl::parse(black_box(input), None).unwrap());
    });

    group.finish();
}

fn bench_parse_complex_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_complex");
    let input =
        "https://user:pass@secure.example.com:8080/path/to/resource?query=value&key=data#section";

    group.bench_function("ars", |b| {
        b.iter(|| ars::Url::parse(black_box(input), None).unwrap());
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| UrlCrate::parse(black_box(input)).unwrap());
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| AdaUrl::parse(black_box(input), None).unwrap());
    });

    group.finish();
}

fn bench_parse_ipv4_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_ipv4");
    let input = "http://192.168.1.1:3000/api";

    group.bench_function("ars", |b| {
        b.iter(|| ars::Url::parse(black_box(input), None).unwrap());
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| UrlCrate::parse(black_box(input)).unwrap());
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| AdaUrl::parse(black_box(input), None).unwrap());
    });

    group.finish();
}

fn bench_parse_ipv6_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_ipv6");
    let input = "http://[2001:db8::1]:8080/path";

    group.bench_function("ars", |b| {
        b.iter(|| ars::Url::parse(black_box(input), None).unwrap());
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| UrlCrate::parse(black_box(input)).unwrap());
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| AdaUrl::parse(black_box(input), None).unwrap());
    });

    group.finish();
}

fn bench_getters_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("getters");
    let input = "https://user:pass@example.com:8080/path?query=value#hash";

    let ars_url = ars::Url::parse(input, None).unwrap();
    let url_crate_url = UrlCrate::parse(input).unwrap();
    let ada_url = AdaUrl::parse(input, None).unwrap();

    group.bench_function("ars", |b| {
        b.iter(|| {
            black_box(ars_url.protocol());
            black_box(ars_url.username());
            black_box(ars_url.password());
            black_box(ars_url.host());
            black_box(ars_url.port());
            black_box(ars_url.pathname());
            black_box(ars_url.search());
            black_box(ars_url.hash());
        });
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| {
            black_box(url_crate_url.scheme());
            black_box(url_crate_url.username());
            black_box(url_crate_url.password());
            black_box(url_crate_url.host_str());
            black_box(url_crate_url.port());
            black_box(url_crate_url.path());
            black_box(url_crate_url.query());
            black_box(url_crate_url.fragment());
        });
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| {
            black_box(ada_url.protocol());
            black_box(ada_url.username());
            black_box(ada_url.password());
            black_box(ada_url.host());
            black_box(ada_url.port());
            black_box(ada_url.pathname());
            black_box(ada_url.search());
            black_box(ada_url.hash());
        });
    });

    group.finish();
}

fn bench_relative_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_relative");
    let input = "/relative/path?query=1";
    let base = "http://example.com/base/";

    group.bench_function("ars", |b| {
        b.iter(|| ars::Url::parse(black_box(input), Some(base)).unwrap());
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| {
            let base_url = UrlCrate::parse(base).unwrap();
            base_url.join(black_box(input)).unwrap()
        });
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| AdaUrl::parse(black_box(input), Some(base)).unwrap());
    });

    group.finish();
}

fn bench_parse_top100urls(c: &mut Criterion) {
    // Ensure we have the latest top100.txt before benchmarking
    ensure_top100_txt();

    // Load URLs from top100.txt
    let urls_content = std::fs::read_to_string("./benchmarks/url-various-datasets/top100.txt")
        .expect("Failed to read top100.txt");
    let urls: Vec<&str> = urls_content.lines().collect();

    let mut group = c.benchmark_group("parse_top100urls");

    group.bench_function("ars", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = ars::Url::parse(black_box(url), None);
            }
        });
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = UrlCrate::parse(black_box(url));
            }
        });
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = AdaUrl::parse(black_box(url), None);
            }
        });
    });

    group.finish();
}

fn bench_parse_wikipedia(c: &mut Criterion) {
    // Ensure we have the latest wikipedia_100k.txt before benchmarking
    ensure_wikipedia_100k_txt();

    // Load URLs from wikipedia_100k.txt
    let urls_content =
        std::fs::read_to_string("./benchmarks/url-various-datasets/wikipedia_100k.txt")
            .expect("Failed to read wikipedia_100k.txt");
    let urls: Vec<&str> = urls_content.lines().collect();

    println!("📊 Benchmarking {} Wikipedia URLs", urls.len());

    let mut group = c.benchmark_group("parse_wikipedia");

    group.bench_function("ars", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = ars::Url::parse(black_box(url), None);
            }
        });
    });

    group.bench_function("url_crate", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = UrlCrate::parse(black_box(url));
            }
        });
    });

    group.bench_function("ada_url", |b| {
        b.iter(|| {
            for url in &urls {
                let _ = AdaUrl::parse(black_box(url), None);
            }
        });
    });

    group.finish();
}

fn bench_can_parse_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("can_parse");

    // Valid URLs
    let valid_simple = "http://example.com/";
    let valid_complex =
        "https://user:pass@secure.example.com:8080/path/to/resource?query=value&key=data#section";

    // Invalid URLs
    let invalid_simple = "not a url";
    let invalid_relative = "/path/without/base";

    group.bench_function("ars_valid_simple", |b| {
        b.iter(|| ars::Url::can_parse(black_box(valid_simple), None));
    });

    group.bench_function("ars_valid_complex", |b| {
        b.iter(|| ars::Url::can_parse(black_box(valid_complex), None));
    });

    group.bench_function("ars_invalid_simple", |b| {
        b.iter(|| ars::Url::can_parse(black_box(invalid_simple), None));
    });

    group.bench_function("ars_invalid_relative", |b| {
        b.iter(|| ars::Url::can_parse(black_box(invalid_relative), None));
    });

    group.bench_function("ada_url_valid_simple", |b| {
        b.iter(|| AdaUrl::can_parse(black_box(valid_simple), None));
    });

    group.bench_function("ada_url_valid_complex", |b| {
        b.iter(|| AdaUrl::can_parse(black_box(valid_complex), None));
    });

    group.bench_function("ada_url_invalid_simple", |b| {
        b.iter(|| AdaUrl::can_parse(black_box(invalid_simple), None));
    });

    group.bench_function("ada_url_invalid_relative", |b| {
        b.iter(|| AdaUrl::can_parse(black_box(invalid_relative), None));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_simple_all,
    bench_parse_complex_all,
    bench_parse_ipv4_all,
    bench_parse_ipv6_all,
    bench_getters_all,
    bench_relative_all,
    bench_parse_top100urls,
    bench_parse_wikipedia,
    bench_can_parse_all
);

criterion_main!(benches);
