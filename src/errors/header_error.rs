pub enum HeaderError {
    Missing(&'static str),
    Invalid(&'static str),
}
