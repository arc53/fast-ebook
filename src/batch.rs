use rayon::prelude::*;

use crate::errors::EpubError;
use crate::model::EpubBook;
use crate::reader::{read_epub_with_options, ReadOptions};

/// Read multiple EPUB files in parallel using Rayon.
/// Returns a Vec of Results — partial failures do not abort the batch.
pub fn read_epubs_parallel(
    paths: &[String],
    opts: &ReadOptions,
    workers: Option<usize>,
) -> Vec<Result<EpubBook, EpubError>> {
    if let Some(n) = workers {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build()
            .expect("Failed to build thread pool");
        pool.install(|| {
            paths
                .par_iter()
                .map(|p| read_epub_with_options(p, opts))
                .collect()
        })
    } else {
        paths
            .par_iter()
            .map(|p| read_epub_with_options(p, opts))
            .collect()
    }
}
