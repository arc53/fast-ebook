pub mod batch;
mod container;
pub mod errors;
pub mod item_type;
mod manifest;
pub mod markdown;
pub mod metadata;
pub mod model;
mod nav;
mod ncx;
pub mod reader;
pub mod spine;
pub mod validation;
pub mod writer;

#[cfg(feature = "python")]
pub mod pybridge;

#[cfg(feature = "python")]
mod python_bindings {
    use pyo3::prelude::*;
    use pyo3::types::PyBytes;

    use crate::{batch, pybridge, reader, writer};

    #[pyfunction]
    #[pyo3(signature = (path, ignore_ncx=false, ignore_nav=false, lazy=false))]
    fn _read_epub(
        path: &str,
        ignore_ncx: bool,
        ignore_nav: bool,
        lazy: bool,
    ) -> PyResult<pybridge::PyEpubBook> {
        let opts = reader::ReadOptions {
            ignore_ncx,
            ignore_nav,
            lazy,
        };
        let book =
            reader::read_epub_with_options(path, &opts).map_err(|e| -> PyErr { e.into() })?;
        Ok(pybridge::PyEpubBook { inner: book })
    }

    #[pyfunction]
    #[pyo3(signature = (data, ignore_ncx=false, ignore_nav=false, lazy=false))]
    fn _read_epub_bytes(
        data: &[u8],
        ignore_ncx: bool,
        ignore_nav: bool,
        lazy: bool,
    ) -> PyResult<pybridge::PyEpubBook> {
        let opts = reader::ReadOptions {
            ignore_ncx,
            ignore_nav,
            lazy,
        };
        let book = reader::read_epub_from_bytes_with_options(data, &opts)
            .map_err(|e| -> PyErr { e.into() })?;
        Ok(pybridge::PyEpubBook { inner: book })
    }

    #[pyfunction]
    fn _write_epub(path: &str, book: &pybridge::PyEpubBook) -> PyResult<()> {
        writer::write_epub(path, &book.inner).map_err(|e| -> PyErr { e.into() })
    }

    #[pyfunction]
    fn _write_epub_bytes<'py>(
        py: Python<'py>,
        book: &pybridge::PyEpubBook,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let data = writer::write_epub_to_bytes(&book.inner).map_err(|e| -> PyErr { e.into() })?;
        Ok(PyBytes::new(py, &data))
    }

    #[pyfunction]
    #[pyo3(signature = (paths, workers=None, ignore_ncx=false, ignore_nav=false))]
    fn _read_epubs(
        py: Python<'_>,
        paths: Vec<String>,
        workers: Option<usize>,
        ignore_ncx: bool,
        ignore_nav: bool,
    ) -> PyResult<Vec<pybridge::PyEpubBook>> {
        let opts = reader::ReadOptions {
            ignore_ncx,
            ignore_nav,
            lazy: false,
        };

        let results = py.allow_threads(|| batch::read_epubs_parallel(&paths, &opts, workers));

        let mut books = Vec::with_capacity(results.len());
        for result in results {
            let book = result.map_err(|e| -> PyErr { e.into() })?;
            books.push(pybridge::PyEpubBook { inner: book });
        }
        Ok(books)
    }

    #[pymodule]
    pub fn _fast_ebook(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(_read_epub, m)?)?;
        m.add_function(wrap_pyfunction!(_read_epub_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(_write_epub, m)?)?;
        m.add_function(wrap_pyfunction!(_write_epub_bytes, m)?)?;
        m.add_function(wrap_pyfunction!(_read_epubs, m)?)?;
        m.add_class::<pybridge::PyEpubBook>()?;
        m.add_class::<pybridge::PyEpubItem>()?;
        m.add_class::<pybridge::PyTocEntry>()?;
        Ok(())
    }
}
