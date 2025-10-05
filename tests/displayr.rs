use displayr::display;

#[display(doc)]
pub enum DataStoreError {
    Disconnect(std::io::Error) = "data store disconnected",
    Redaction(String) = "the data for key `{_0}` is not available",
    InvalidHeader {
        expected: String,
        found: String,
    } = "invalid header (expected {expected:?}, found {found:?})",
    Unknown = "unknown data store error",
}
