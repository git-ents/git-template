//! `.gittemplate` configuration file parsing.

use std::path::Path;

/// Top-level structure of a `.gittemplate` file.
#[derive(serde::Deserialize)]
pub struct TemplateConfig {
    /// The `[template]` section.
    pub template: TemplateSection,
}

/// Contents of the `[template]` section.
#[derive(serde::Deserialize)]
pub struct TemplateSection {
    /// Glob patterns selecting which files are rendered as templates.
    /// When absent, all files are rendered.
    pub files: Option<Vec<String>>,

    /// Variable declarations, each corresponding to a `[[var]]` entry.
    #[serde(rename = "var", default)]
    pub vars: Vec<TemplateVar>,
}

/// A single template variable declaration.
#[derive(serde::Deserialize, Clone)]
pub struct TemplateVar {
    /// Variable name as it appears in `{{ name }}` expressions.
    pub name: String,
    /// Human-readable description shown in the editor prompt.
    pub description: String,
    /// Optional default value pre-filled in the editor.
    #[serde(default)]
    pub default: String,
}

/// Load `.gittemplate` from `workdir`, returning `None` if the file does not exist.
pub fn load(workdir: &Path) -> Result<Option<TemplateConfig>, Box<dyn std::error::Error>> {
    let path = workdir.join(".gittemplate");
    if !path.exists() {
        return Ok(None);
    }
    let src = std::fs::read_to_string(path)?;
    Ok(Some(toml::from_str(&src)?))
}
