//! Project-local syntax search. Private source never shares the bundled KB index.
use super::{digest, Engine, IndexReport, RetrievalConfig, SearchFilter, SearchHit};
use std::{path::Path, sync::atomic::AtomicBool};

pub fn search(
    root: &Path,
    cache: &Path,
    query: &str,
    cancel: &AtomicBool,
) -> Result<(IndexReport, Vec<SearchHit>), String> {
    let root = dunce::canonicalize(root).map_err(|e| e.to_string())?;
    let index = cache
        .join("private-code-index")
        .join(digest(root.to_string_lossy().as_bytes()));
    std::fs::create_dir_all(&index).map_err(|e| e.to_string())?;
    // Syntax/code search uses lexical ranking. It does not start a new model or
    // infer semantic references, and changes remain scoped to this project's DB.
    let engine = Engine::open_project(
        &index,
        RetrievalConfig {
            dense_enabled: false,
            technical_expansion: false, // Public terminology must not change private symbol search.
            ..Default::default()
        },
    )?;
    let report = engine.ingest(&root, cancel, |_| {
        Err("Project lexical index must not embed".into())
    })?;
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("Project search cancelled".into());
    }
    let hits = engine.search(
        query,
        None,
        &SearchFilter {
            include_unknown: true,
            ..Default::default()
        },
    )?;
    Ok((report, hits))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incremental_private_indexes_cannot_mix_projects_or_index_hidden_context() {
        let base = tempfile::tempdir().unwrap();
        let cache = base.path().join("cache");
        let a = base.path().join("a");
        let b = base.path().join("b");
        for root in [&a, &b] {
            std::fs::create_dir_all(root.join("main")).unwrap();
        }
        std::fs::write(
            a.join("main/main.c"),
            "int private_alpha(void) {return 1;}\r\n",
        )
        .unwrap();
        std::fs::write(
            b.join("main/main.c"),
            "int private_beta(void) {return 2;}\n",
        )
        .unwrap();
        std::fs::write(a.join("credentials.txt"), "private_secret").unwrap();
        let cancel = AtomicBool::new(false);
        let (first, hits) = search(&a, &cache, "private_alpha", &cancel).unwrap();
        assert_eq!(first.files_updated, 1);
        assert!(!hits.is_empty());
        assert!(hits
            .iter()
            .all(|h| !h.bundled && h.parser.starts_with("tree-sitter-c")));
        let (again, _) = search(&a, &cache, "private_secret", &cancel).unwrap();
        assert_eq!(again.files_unchanged, 1);
        assert!(search(&b, &cache, "private_alpha", &cancel)
            .unwrap()
            .1
            .is_empty());
        std::fs::write(a.join("main/main.c"), "int replacement(void) {return 3;}").unwrap();
        assert_eq!(
            search(&a, &cache, "replacement", &cancel)
                .unwrap()
                .0
                .files_updated,
            1
        );
        assert!(search(&a, &cache, "private_alpha", &cancel)
            .unwrap()
            .1
            .is_empty());
        std::fs::remove_file(a.join("main/main.c")).unwrap();
        assert_eq!(
            search(&a, &cache, "replacement", &cancel)
                .unwrap()
                .0
                .files_deleted,
            1
        );
        assert!(search(&a, &cache, "replacement", &cancel)
            .unwrap()
            .1
            .is_empty());
    }
}
