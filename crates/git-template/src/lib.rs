//! Template repository support for Git.

use git_vendor::Vendor as _;

use git2::Error;

type Result<T = ()> = std::result::Result<T, Error>;

/// Temp.
pub trait TemplateRepository {
    fn from_template() -> Result {
        Ok(())
    }
}
