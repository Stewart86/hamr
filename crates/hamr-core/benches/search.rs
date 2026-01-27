//! Benchmarks for the search engine.
//!
//! Run with: cargo bench -p hamr-core
//! Results are saved to target/criterion/

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use hamr_core::search::{SearchEngine, Searchable, SearchableSource};

fn make_searchable(id: &str, name: &str, keywords: Vec<&str>) -> Searchable {
    Searchable {
        id: id.to_string(),
        name: name.to_string(),
        keywords: keywords.into_iter().map(String::from).collect(),
        source: SearchableSource::Plugin { id: id.to_string() },
        is_history_term: false,
    }
}

fn generate_app_searchables(count: usize) -> Vec<Searchable> {
    let app_names = [
        ("firefox", "Firefox", vec!["browser", "web", "mozilla"]),
        ("chrome", "Google Chrome", vec!["browser", "web", "google"]),
        (
            "vscode",
            "Visual Studio Code",
            vec!["editor", "code", "ide", "microsoft"],
        ),
        ("terminal", "Terminal", vec!["console", "shell", "bash"]),
        ("nautilus", "Files", vec!["file manager", "explorer"]),
        ("spotify", "Spotify", vec!["music", "audio", "streaming"]),
        ("discord", "Discord", vec!["chat", "voice", "gaming"]),
        ("slack", "Slack", vec!["chat", "work", "messaging"]),
        (
            "thunderbird",
            "Thunderbird",
            vec!["email", "mail", "mozilla"],
        ),
        ("gimp", "GIMP", vec!["image", "editor", "graphics"]),
        ("vlc", "VLC Media Player", vec!["video", "audio", "media"]),
        (
            "libreoffice",
            "LibreOffice Writer",
            vec!["document", "word", "office"],
        ),
        ("inkscape", "Inkscape", vec!["vector", "graphics", "svg"]),
        ("blender", "Blender", vec!["3d", "modeling", "animation"]),
        ("obs", "OBS Studio", vec!["streaming", "recording", "video"]),
        ("telegram", "Telegram", vec!["chat", "messaging", "social"]),
        ("signal", "Signal", vec!["chat", "messaging", "secure"]),
        ("zoom", "Zoom", vec!["video", "meeting", "conference"]),
        ("steam", "Steam", vec!["games", "gaming", "valve"]),
        ("lutris", "Lutris", vec!["games", "gaming", "wine"]),
    ];

    let mut searchables = Vec::with_capacity(count);
    for i in 0..count {
        let idx = i % app_names.len();
        let (id, name, keywords) = &app_names[idx];
        let unique_id = format!("{id}_{i}");
        searchables.push(make_searchable(&unique_id, name, keywords.clone()));
    }
    searchables
}

fn bench_search_basic(c: &mut Criterion) {
    let searchables = generate_app_searchables(100);
    let mut engine = SearchEngine::new();

    c.bench_function("search_basic_100", |b| {
        b.iter(|| engine.search(black_box("fire"), black_box(&searchables)));
    });
}

fn bench_search_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_scaling");

    for size in &[100, 500, 1000, 5000, 10000] {
        let searchables = generate_app_searchables(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut engine = SearchEngine::new();
            b.iter(|| engine.search(black_box("chrome"), black_box(&searchables)));
        });
    }
    group.finish();
}

fn bench_search_queries(c: &mut Criterion) {
    let searchables = generate_app_searchables(1000);
    let mut group = c.benchmark_group("search_queries");

    let queries = [
        ("short", "vs"),
        ("medium", "visual"),
        ("long", "visual studio code"),
        ("partial", "libr"),
        ("keyword", "browser"),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::new("query", name), query, |b, q| {
            let mut engine = SearchEngine::new();
            b.iter(|| engine.search(black_box(q), black_box(&searchables)));
        });
    }
    group.finish();
}

fn bench_name_match_bonus(c: &mut Criterion) {
    let mut group = c.benchmark_group("name_match_bonus");

    let cases = [
        ("exact", "firefox", "firefox"),
        ("prefix_short", "fire", "firefox"),
        ("prefix_long", "firefo", "firefox"),
        ("no_match", "chrome", "firefox"),
        ("case_insensitive", "FIREFOX", "firefox"),
    ];

    for (name, query, target) in cases {
        group.bench_with_input(
            BenchmarkId::new("case", name),
            &(query, target),
            |b, &(q, t)| b.iter(|| SearchEngine::name_match_bonus(black_box(q), black_box(t))),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_search_basic,
    bench_search_scaling,
    bench_search_queries,
    bench_name_match_bonus
);
criterion_main!(benches);
