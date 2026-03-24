use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

mod builtin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThemeKind {
    Api,
    Ai,
    Local,
}

impl ThemeKind {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "api" => Some(Self::Api),
            "ai" => Some(Self::Ai),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Palette {
    pub background: Option<String>,
    pub text: Option<String>,
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub muted: Option<String>,
    pub quote_background: Option<String>,
    pub code_background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Typography {
    pub font_family: Option<String>,
    pub body_size: Option<String>,
    pub line_height: Option<String>,
    pub letter_spacing: Option<String>,
    pub link_underline: Option<bool>,
    pub heading_align: Option<String>,
    pub h1_size: Option<String>,
    pub h2_size: Option<String>,
    pub h3_size: Option<String>,
    pub h1_background: Option<String>,
    pub h1_radius: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerLayout {
    pub padding: Option<String>,
    pub max_width: Option<String>,
    pub center: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardLayout {
    pub enabled: Option<bool>,
    pub padding: Option<String>,
    pub radius: Option<String>,
    pub background: Option<String>,
    pub border: Option<String>,
    pub shadow: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Decorations {
    pub background_texture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Layout {
    pub container: Option<ContainerLayout>,
    pub card: Option<CardLayout>,
    pub decorations: Option<Decorations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub colors: Option<HashMap<String, String>>,
    pub palette: Option<Palette>,
    pub typography: Option<Typography>,
    pub layout: Option<Layout>,
    pub api_theme: Option<String>,
    pub prompt: Option<String>,
}

impl Theme {
    pub fn kind(&self) -> ThemeKind {
        self.r#type
            .as_deref()
            .and_then(ThemeKind::parse)
            .unwrap_or(ThemeKind::Local)
    }
}

#[derive(Debug, Default)]
pub struct ThemeManager {
    themes: HashMap<String, Theme>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        for theme in builtin::builtin_themes() {
            themes.insert(theme.name.clone(), theme);
        }
        Self { themes }
    }

    pub fn load_dirs(&mut self, dirs: &[PathBuf]) -> Result<(), ThemeError> {
        for dir in dirs {
            self.load_from_dir(dir)?;
        }
        Ok(())
    }

    pub fn load_from_dir(&mut self, dir: &Path) -> Result<(), ThemeError> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(ThemeError::ReadDir(dir.to_path_buf(), err)),
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some("yaml") | Some("yml")
                )
            })
            .collect();
        paths.sort();

        for path in paths {
            let raw =
                fs::read_to_string(&path).map_err(|err| ThemeError::ReadFile(path.clone(), err))?;
            let theme: Theme = serde_yaml::from_str(&raw)
                .map_err(|err| ThemeError::ParseYaml(path.clone(), err))?;
            if theme.name.trim().is_empty() {
                return Err(ThemeError::InvalidThemeName(path));
            }
            self.themes.insert(theme.name.clone(), theme);
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }

    pub fn list(&self) -> Vec<&Theme> {
        let mut themes: Vec<&Theme> = self.themes.values().collect();
        themes.sort_by(|a, b| {
            let a_kind = a.kind();
            let b_kind = b.kind();
            match a_kind.cmp(&b_kind) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            }
        });
        themes
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("read theme directory {0:?}: {1}")]
    ReadDir(PathBuf, #[source] std::io::Error),
    #[error("read theme file {0:?}: {1}")]
    ReadFile(PathBuf, #[source] std::io::Error),
    #[error("parse theme yaml {0:?}: {1}")]
    ParseYaml(PathBuf, #[source] serde_yaml::Error),
    #[error("theme name is required in {0:?}")]
    InvalidThemeName(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_builtin_themes() {
        let tm = ThemeManager::new();
        assert_eq!(tm.themes.len(), 48);
        for name in [
            "default",
            "bytedance",
            "apple",
            "sports",
            "chinese",
            "cyber",
            "wechat-native",
            "nyt-classic",
            "github-readme",
            "sspai-red",
            "mint-fresh",
            "sunset-amber",
            "ink-minimal",
            "minimal-gold",
            "focus-blue",
            "elegant-gray",
            "bold-red",
        ] {
            assert!(tm.get(name).is_some(), "missing theme {name}");
        }
    }
}
