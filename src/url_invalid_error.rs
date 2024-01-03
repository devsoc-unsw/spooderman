
#[derive(Debug)]
pub struct UrlInvalidError;

impl std::fmt::Display for UrlInvalidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The supplied URL is not valid!")
    }
}

impl std::error::Error for UrlInvalidError {}
