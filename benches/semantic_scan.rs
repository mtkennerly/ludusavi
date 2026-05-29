use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use ludusavi::path::StrictPath;
use ludusavi::resource::manifest::Store;
use ludusavi::scan::saves::ScanOrigin;
use ludusavi::semantic::SemanticPath;
use ludusavi::semantic::convert::{
    KnownFolders, derive_from_manifest_origin, windows_physical_to_semantic, wine_physical_to_semantic,
};

fn make_known_folders() -> KnownFolders {
    KnownFolders {
        saved_games: Some("C:/Users/Alice/Saved Games".to_string()),
        documents: Some("C:/Users/Alice/Documents".to_string()),
        local_app_data: Some("C:/Users/Alice/AppData/Local".to_string()),
        app_data: Some("C:/Users/Alice/AppData/Roaming".to_string()),
        public: Some("C:/Users/Public".to_string()),
        program_data: Some("C:/ProgramData".to_string()),
        windows: Some("C:/Windows".to_string()),
        user_profile: Some("C:/Users/Alice".to_string()),
    }
}

fn bench_parse(c: &mut Criterion) {
    c.bench_function("semantic_parse", |b| {
        b.iter(|| {
            SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
        })
    });
}

fn bench_serialize(c: &mut Criterion) {
    let sp = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
    c.bench_function("semantic_serialize", |b| {
        b.iter(|| {
            sp.serialize();
        })
    });
}

fn bench_storage_path(c: &mut Criterion) {
    let sp = SemanticPath::parse("<winDocuments>/Game/save.dat").unwrap();
    c.bench_function("semantic_storage_path", |b| {
        b.iter(|| {
            sp.storage_path();
        })
    });
}

fn bench_windows_to_semantic(c: &mut Criterion) {
    let kf = make_known_folders();
    let paths = [
        StrictPath::new("C:/Users/Alice/Documents/Game/save.dat"),
        StrictPath::new("C:/Users/Alice/AppData/Roaming/Game/config.ini"),
        StrictPath::new("C:/Users/Alice/AppData/Local/Game/cache.dat"),
        StrictPath::new("C:/ProgramData/Game/telemetry.dat"),
        StrictPath::new("D:/Games/save.dat"),
    ];

    let mut group = c.benchmark_group("windows_physical_to_semantic");
    for (i, path) in paths.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("path", i), path, |b, p| {
            b.iter(|| {
                windows_physical_to_semantic(p, &kf);
            })
        });
    }
    group.finish();
}

fn bench_wine_to_semantic(c: &mut Criterion) {
    let prefix = StrictPath::new("/home/deck/Prefixes/Game");
    let paths = [
        StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game/save.dat"),
        StrictPath::new("/home/deck/Prefixes/Game/drive_c/users/steamuser/AppData/Roaming/Game/config.ini"),
        StrictPath::new("/home/deck/Prefixes/Game/drive_c/ProgramData/Game/telemetry.dat"),
        StrictPath::new("/home/deck/Prefixes/Game/drive_d/Games/save.dat"),
    ];

    let mut group = c.benchmark_group("wine_physical_to_semantic");
    for (i, path) in paths.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("path", i), path, |b, p| {
            b.iter(|| {
                wine_physical_to_semantic(p, &prefix, "steamuser");
            })
        });
    }
    group.finish();
}

fn bench_manifest_derive(c: &mut Criterion) {
    let origins = [
        ScanOrigin {
            manifest_path: "<winDocuments>/Remedy/Alan Wake".to_string(),
            store: Store::Other,
            expanded_prefix: "C:/Users/Alice/Documents".to_string(),
            matched_prefix_len: 25,
            tail: "Remedy/Alan Wake/save.dat".to_string(),
        },
        ScanOrigin {
            manifest_path: "<root>/userdata/<storeUserId>/<storeGameId>/remote".to_string(),
            store: Store::Steam,
            expanded_prefix: "C:/Program Files (x86)/Steam".to_string(),
            matched_prefix_len: 34,
            tail: "userdata/12345/67890/remote/save.dat".to_string(),
        },
    ];

    let mut group = c.benchmark_group("manifest_derive");
    for (i, origin) in origins.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("origin", i), origin, |b, o| {
            b.iter(|| {
                derive_from_manifest_origin(o);
            })
        });
    }
    group.finish();
}

fn bench_batch_scan_simulation(c: &mut Criterion) {
    let kf = make_known_folders();
    let prefix = StrictPath::new("/home/deck/Prefixes/Game");

    // Simulate scanning 500 games worth of paths
    let windows_paths: Vec<StrictPath> = (0..500)
        .map(|i| StrictPath::new(format!("C:/Users/Alice/Documents/Game{}/save.dat", i)))
        .collect();

    let wine_paths: Vec<StrictPath> = (0..500)
        .map(|i| {
            StrictPath::new(format!(
                "/home/deck/Prefixes/Game/drive_c/users/steamuser/Documents/Game{}/save.dat",
                i
            ))
        })
        .collect();

    c.bench_function("batch_500_windows", |b| {
        b.iter(|| {
            for path in &windows_paths {
                windows_physical_to_semantic(path, &kf);
            }
        })
    });

    c.bench_function("batch_500_wine", |b| {
        b.iter(|| {
            for path in &wine_paths {
                wine_physical_to_semantic(path, &prefix, "steamuser");
            }
        })
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_serialize,
    bench_storage_path,
    bench_windows_to_semantic,
    bench_wine_to_semantic,
    bench_manifest_derive,
    bench_batch_scan_simulation,
);
criterion_main!(benches);
