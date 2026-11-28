// Usage: cargo run --release -p skopos-core --example search_bench <archive.jsonl.zst> <query> [runs]
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let runs: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(3);
    for _ in 0..runs {
        let t = Instant::now();
        let f = std::fs::File::open(&a[1]).unwrap();
        let r = skopos_core::search::search_archive(f, &a[2]).unwrap();
        println!(
            "{{\"documents\":{},\"parsed\":{},\"items_scanned\":{},\"hits\":{},\"seconds\":{:.2}}}",
            r.documents,
            r.documents_parsed,
            r.items_scanned,
            r.hits.len(),
            t.elapsed().as_secs_f64()
        );
    }
}
