//! Editor-based template variable prompt.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;

/// A template variable to present in the editor prompt.
pub struct Var {
    /// Variable name.
    pub name: String,
    /// Optional description shown as a comment above the entry.
    pub description: Option<String>,
    /// Default value pre-filled in the editor.
    pub default: String,
}

/// Write a parameter prompt to `$GIT_DIR/TEMPLATE_PARAMS`, open `$GIT_EDITOR`,
/// and parse the result into a `name → value` map.
pub fn prompt(vars: &[Var], git_dir: &Path) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let path = git_dir.join("TEMPLATE_PARAMS");

    {
        let mut f = std::fs::File::create(&path)?;
        writeln!(f, "# git template init")?;
        writeln!(f, "# Fill in values below. Lines starting with '#' are ignored.")?;
        writeln!(f, "# Format:  key: value")?;
        for var in vars {
            writeln!(f)?;
            if let Some(desc) = &var.description {
                writeln!(f, "# {}: {}", var.name, desc)?;
            }
            writeln!(f, "{}: {}", var.name, var.default)?;
        }
    }

    let editor = std::env::var("GIT_EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| format!("failed to launch editor '{}': {}", editor, e))?;

    let content = std::fs::read_to_string(&path)?;
    let _ = std::fs::remove_file(&path);

    if !status.success() {
        return Err(format!("editor exited with {}", status).into());
    }

    let mut values = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once(':') {
            values.insert(key.trim().to_string(), val.trim().to_string());
        }
    }

    Ok(values)
}
