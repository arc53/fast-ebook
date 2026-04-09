use std::fmt;

#[derive(Debug)]
pub enum EpubError {
    Zip(zip::result::ZipError),
    Xml(roxmltree::Error),
    InvalidMimetype(String),
    MissingContainer,
    MissingRootfile,
    MissingOpf(String),
    InvalidOpf(String),
    ItemNotFound(String),
    Io(std::io::Error),
    WriteError(String),
    MissingIdentifier,
    MissingTitle,
    MissingLanguage,
    EmptySpine,
}

impl fmt::Display for EpubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EpubError::Zip(e) => write!(f, "ZIP error: {e}"),
            EpubError::Xml(e) => write!(f, "XML parsing error: {e}"),
            EpubError::InvalidMimetype(m) => write!(f, "Invalid mimetype: {m}"),
            EpubError::MissingContainer => write!(f, "Missing META-INF/container.xml"),
            EpubError::MissingRootfile => write!(f, "No rootfile found in container.xml"),
            EpubError::MissingOpf(p) => write!(f, "OPF file not found in archive: {p}"),
            EpubError::InvalidOpf(msg) => write!(f, "Invalid OPF: {msg}"),
            EpubError::ItemNotFound(id) => write!(f, "Item not found: {id}"),
            EpubError::Io(e) => write!(f, "IO error: {e}"),
            EpubError::WriteError(msg) => write!(f, "Write error: {msg}"),
            EpubError::MissingIdentifier => write!(f, "Book must have an identifier set"),
            EpubError::MissingTitle => write!(f, "Book must have a title set"),
            EpubError::MissingLanguage => write!(f, "Book must have a language set"),
            EpubError::EmptySpine => write!(f, "Book must have at least one spine entry"),
        }
    }
}

#[cfg(feature = "python")]
impl From<EpubError> for pyo3::PyErr {
    fn from(err: EpubError) -> pyo3::PyErr {
        pyo3::exceptions::PyValueError::new_err(err.to_string())
    }
}

impl From<zip::result::ZipError> for EpubError {
    fn from(e: zip::result::ZipError) -> Self {
        EpubError::Zip(e)
    }
}

impl From<roxmltree::Error> for EpubError {
    fn from(e: roxmltree::Error) -> Self {
        EpubError::Xml(e)
    }
}

impl From<std::io::Error> for EpubError {
    fn from(e: std::io::Error) -> Self {
        EpubError::Io(e)
    }
}
