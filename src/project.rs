//! Smart frontend and web project detection for contextual downloads.

use std::path::{Path, PathBuf};
use serde_json::Value;

/// Web/frontend framework or runtime environment detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    React,
    NextJs,
    Vue,
    Nuxt,
    Svelte,
    Astro,
    Angular,
    Vite,
    NodeWeb,
}

impl ProjectKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProjectKind::React => "React",
            ProjectKind::NextJs => "Next.js",
            ProjectKind::Vue => "Vue",
            ProjectKind::Nuxt => "Nuxt",
            ProjectKind::Svelte => "Svelte",
            ProjectKind::Astro => "Astro",
            ProjectKind::Angular => "Angular",
            ProjectKind::Vite => "Vite",
            ProjectKind::NodeWeb => "Web/Node",
        }
    }
}

/// Project context containing root folder and determined target SVG directory.
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub kind: ProjectKind,
    pub target_dir: PathBuf,
}

impl ProjectContext {
    pub fn root_name(&self) -> String {
        self.root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string())
    }
}

/// Detect project context starting from current working directory.
pub fn find_project_context() -> Option<ProjectContext> {
    let current = std::env::current_dir().ok()?;
    let home = dirs::home_dir();
    find_project_context_from(&current, home.as_deref())
}

/// Detect project context from a specified directory, respecting home dir boundary.
pub fn find_project_context_from(start_dir: &Path, home_dir: Option<&Path>) -> Option<ProjectContext> {
    // If running directly in home/root directory (e.g. ~, /Users/username),
    // do not treat as a project folder.
    if let Some(home) = home_dir {
        if start_dir == home {
            return None;
        }
    }

    let mut current = Some(start_dir);
    while let Some(dir) = current {
        // Stop if we hit home dir or filesystem root
        if let Some(home) = home_dir {
            if dir == home {
                break;
            }
        }
        if dir.parent().is_none() {
            break;
        }

        if let Some(ctx) = inspect_directory(dir) {
            return Some(ctx);
        }

        current = dir.parent();
    }

    None
}

/// Inspect a single directory for frontend/project files.
fn inspect_directory(dir: &Path) -> Option<ProjectContext> {
    let pkg_json_path = dir.join("package.json");
    let has_pkg_json = pkg_json_path.is_file();

    // Check config files if no package.json or to supplement package.json
    let has_next_config = dir.join("next.config.js").exists()
        || dir.join("next.config.ts").exists()
        || dir.join("next.config.mjs").exists();
    let has_vite_config = dir.join("vite.config.js").exists()
        || dir.join("vite.config.ts").exists()
        || dir.join("vite.config.mjs").exists();
    let has_nuxt_config = dir.join("nuxt.config.js").exists()
        || dir.join("nuxt.config.ts").exists();
    let has_astro_config = dir.join("astro.config.mjs").exists()
        || dir.join("astro.config.ts").exists();
    let has_svelte_config = dir.join("svelte.config.js").exists()
        || dir.join("svelte.config.ts").exists();

    if !has_pkg_json
        && !has_next_config
        && !has_vite_config
        && !has_nuxt_config
        && !has_astro_config
        && !has_svelte_config
    {
        return None;
    }

    let kind = if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
        detect_framework_from_package_json(&content, has_next_config, has_vite_config, has_nuxt_config, has_astro_config, has_svelte_config)
    } else if has_next_config {
        ProjectKind::NextJs
    } else if has_nuxt_config {
        ProjectKind::Nuxt
    } else if has_astro_config {
        ProjectKind::Astro
    } else if has_svelte_config {
        ProjectKind::Svelte
    } else if has_vite_config {
        ProjectKind::Vite
    } else {
        ProjectKind::NodeWeb
    };

    let target_dir = resolve_project_target_dir(dir);

    Some(ProjectContext {
        root: dir.to_path_buf(),
        kind,
        target_dir,
    })
}

/// Detect framework from package.json JSON string.
fn detect_framework_from_package_json(
    content: &str,
    has_next_cfg: bool,
    has_vite_cfg: bool,
    has_nuxt_cfg: bool,
    has_astro_cfg: bool,
    has_svelte_cfg: bool,
) -> ProjectKind {
    if let Ok(v) = serde_json::from_str::<Value>(content) {
        let has_dep = |name: &str| -> bool {
            v.get("dependencies")
                .and_then(|d| d.get(name))
                .is_some()
                || v.get("devDependencies")
                    .and_then(|d| d.get(name))
                    .is_some()
                || v.get("peerDependencies")
                    .and_then(|d| d.get(name))
                    .is_some()
        };

        if has_dep("next") || has_next_cfg {
            return ProjectKind::NextJs;
        }
        if has_dep("nuxt") || has_nuxt_cfg {
            return ProjectKind::Nuxt;
        }
        if has_dep("astro") || has_astro_cfg {
            return ProjectKind::Astro;
        }
        if has_dep("@sveltejs/kit") || has_dep("svelte") || has_svelte_cfg {
            return ProjectKind::Svelte;
        }
        if has_dep("@angular/core") {
            return ProjectKind::Angular;
        }
        if has_dep("vue") || has_dep("vue-router") {
            return ProjectKind::Vue;
        }
        if has_dep("react") || has_dep("react-dom") || has_dep("react-scripts") {
            return ProjectKind::React;
        }
        if has_dep("vite") || has_vite_cfg {
            return ProjectKind::Vite;
        }
    }

    if has_next_cfg {
        ProjectKind::NextJs
    } else if has_nuxt_cfg {
        ProjectKind::Nuxt
    } else if has_astro_cfg {
        ProjectKind::Astro
    } else if has_svelte_cfg {
        ProjectKind::Svelte
    } else if has_vite_cfg {
        ProjectKind::Vite
    } else {
        ProjectKind::NodeWeb
    }
}

/// Determine where SVGs should be downloaded within the project root.
///
/// Priority:
/// 1. `src/images` (if exists)
/// 2. `src/assets/images` (if exists)
/// 3. `src/assets/icons` (if exists)
/// 4. `src/icons` (if exists)
/// 5. `public/images` (if exists)
/// 6. `public/icons` (if exists)
/// 7. If `src` directory exists -> `src/images`
/// 8. If `public` directory exists (and no `src`) -> `public/images`
/// 9. Fallback -> `src/images`
pub fn resolve_project_target_dir(project_root: &Path) -> PathBuf {
    let p_src_images = project_root.join("src").join("images");
    if p_src_images.is_dir() {
        return p_src_images;
    }

    let p_src_assets_images = project_root.join("src").join("assets").join("images");
    if p_src_assets_images.is_dir() {
        return p_src_assets_images;
    }

    let p_src_assets_icons = project_root.join("src").join("assets").join("icons");
    if p_src_assets_icons.is_dir() {
        return p_src_assets_icons;
    }

    let p_src_icons = project_root.join("src").join("icons");
    if p_src_icons.is_dir() {
        return p_src_icons;
    }

    let p_public_images = project_root.join("public").join("images");
    if p_public_images.is_dir() {
        return p_public_images;
    }

    let p_public_icons = project_root.join("public").join("icons");
    if p_public_icons.is_dir() {
        return p_public_icons;
    }

    let p_src = project_root.join("src");
    if p_src.is_dir() {
        return p_src_images;
    }

    let p_public = project_root.join("public");
    if p_public.is_dir() {
        return p_public_images;
    }

    p_src_images
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn home_dir_is_not_detected_as_project() {
        let temp = tempdir().unwrap();
        let home = temp.path().join("userhome");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("package.json"), r#"{"dependencies":{"react":"^18.0.0"}}"#).unwrap();

        let ctx = find_project_context_from(&home, Some(&home));
        assert!(ctx.is_none());
    }

    #[test]
    fn detects_react_project_with_src() {
        let temp = tempdir().unwrap();
        let home = temp.path().join("userhome");
        let proj = home.join("my-react-app");
        let src = proj.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(proj.join("package.json"), r#"{"dependencies":{"react":"^18.0.0"}}"#).unwrap();

        let ctx = find_project_context_from(&proj, Some(&home)).expect("should detect");
        assert_eq!(ctx.kind, ProjectKind::React);
        assert_eq!(ctx.target_dir, proj.join("src").join("images"));
    }

    #[test]
    fn detects_nextjs_project_with_public_images() {
        let temp = tempdir().unwrap();
        let home = temp.path().join("userhome");
        let proj = home.join("my-portfolio");
        let public_images = proj.join("public").join("images");
        std::fs::create_dir_all(&public_images).unwrap();
        std::fs::write(proj.join("package.json"), r#"{"dependencies":{"next":"14.0.0"}}"#).unwrap();

        let ctx = find_project_context_from(&proj, Some(&home)).expect("should detect");
        assert_eq!(ctx.kind, ProjectKind::NextJs);
        assert_eq!(ctx.target_dir, public_images);
    }

    #[test]
    fn walks_upwards_from_subdirectories() {
        let temp = tempdir().unwrap();
        let home = temp.path().join("userhome");
        let proj = home.join("my-app");
        let nested = proj.join("components").join("sections").join("Experience");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(proj.join("src")).unwrap();
        std::fs::write(proj.join("package.json"), r#"{"dependencies":{"vue":"^3.0.0"}}"#).unwrap();

        let ctx = find_project_context_from(&nested, Some(&home)).expect("should find root");
        assert_eq!(ctx.kind, ProjectKind::Vue);
        assert_eq!(ctx.root, proj);
        assert_eq!(ctx.target_dir, proj.join("src").join("images"));
    }
}
