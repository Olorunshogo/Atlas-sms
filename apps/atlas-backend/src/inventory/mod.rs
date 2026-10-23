/*!
 * The central inventory transaction service. Every stock change in the
 * system - sales, voids, returns, goods receipts, adjustments, transfers -
 * goes through `service::post`, so unit conversion, locking and the
 * negative-stock policy live in exactly one place.
 */
pub mod batches;
pub mod models;
pub mod service;

pub use models::{MovementType, NegativeStockPolicy, Posting, PostingLine, StockMovement};

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Guards the "one writer" rule: no module outside `src/inventory` may insert ledger rows directly.
    #[test]
    fn only_the_inventory_service_writes_stock_movements() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        visit(&src, &mut |path, contents| {
            let in_inventory = path.components().any(|c| c.as_os_str() == "inventory");
            if !in_inventory && contents.contains("INSERT INTO atlas.stock_movements") {
                offenders.push(path.display().to_string());
            }
        });
        assert!(offenders.is_empty(), "direct stock_movements inserts in: {:?}", offenders);
    }

    fn visit(dir: &Path, f: &mut dyn FnMut(&Path, &str)) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, f);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let contents = std::fs::read_to_string(&path).unwrap();
                f(&path, &contents);
            }
        }
    }
}
