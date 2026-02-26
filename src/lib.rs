use mdbook_preprocessor::book::BookItem;
use mdbook_preprocessor::errors::Error;
use mdbook_preprocessor::errors::Result;
use mdbook_preprocessor::parse_input;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext, book::Book};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::exit;
use tracing::debug;
use tracing::error;
use tracing::warn;

pub mod cli;

pub struct Processor;

#[derive(Deserialize)]
/// Used for parsing a subset of Cargo.toml, to get the package name and version for annotation purposes. We support both the `[package]` and `[workspace]` tables, and prefer the former if both are present.
struct CargoToml {
    #[serde(default)]
    package: Option<CargoPackage>,
    #[serde(default)]
    workspace: Option<CargoPackage>,
}

impl CargoToml {
    /// Falls back to worksapce name and version if package name and version are not found, which allows this to work in both workspace and non-workspace projects without any additional configuration.
    pub fn package(&self) -> Option<&CargoPackage> {
        self.package.as_ref().or(self.workspace.as_ref())
    }

    pub fn name(&self) -> Option<&str> {
        self.package().map(|pkg| pkg.name.as_str())
    }

    pub fn version(&self) -> Option<&str> {
        self.package().map(|pkg| pkg.version.as_str())
    }
}

#[derive(Deserialize)]
struct CargoPackage {
    pub name: String,
    pub version: String,
}

#[derive(Debug)]
pub struct Config {
    /// Defaults to 10 if unset, and is used to determine how many characters of the git commit hash to include in the annotation
    pub commit_characters: usize,
    /// Defaults to "../" if unset, and is used to determine where to look for the Cargo.toml and .git directories. This should typically be set to the root of the workspace, so that it works correctly in both workspace and non-workspace projects.
    pub workspace_dir: PathBuf,
    /// Defaults to "../" if unset, and is used to determine where to look for the .git directory. This should typically be set to the root of the workspace, so that it works correctly in both workspace and non-workspace projects.
    pub git_dir: PathBuf,
    /// Defaults to true if unset, and determines whether to include the package name in the annotation
    pub package_name: bool,
    /// Defaults to true if unset, and determines whether to include the package version in the annotation
    pub package_version: bool,
    /// Defaults to true if unset, and determines whether to include the git commit in the annotation
    pub git_commit: bool,
}

impl TryFrom<&PreprocessorContext> for Config {
    type Error = Error;

    fn try_from(ctx: &PreprocessorContext) -> Result<Self> {
        let cfg_key = |key| format!("preprocessor.{}.{}", "build-annotations", key);
        Ok(Config {
            commit_characters: ctx.config.get(&cfg_key("commit_characters"))?.unwrap_or(10),
            workspace_dir: ctx
                .config
                .get(&cfg_key("workspace_dir"))?
                .unwrap_or("../".into()),
            git_dir: ctx.config.get(&cfg_key("git_dir"))?.unwrap_or("../".into()),
            package_name: ctx.config.get(&cfg_key("package_name"))?.unwrap_or(true),
            package_version: ctx.config.get(&cfg_key("package_version"))?.unwrap_or(true),
            git_commit: ctx.config.get(&cfg_key("git_commit"))?.unwrap_or(true),
        })
    }
}

impl Processor {
    /// does the actual work of modifying the book, by appending the given footer to the end of each chapter. This is called from `run` after we've determined the footer text, which includes the package name, version, and git commit.
    fn handle_bookitem(&self, item: &mut BookItem, footer: &str) {
        if let BookItem::Chapter(ref mut chapter) = *item {
            chapter.content.push_str(footer);
        }
    }
}

impl Preprocessor for Processor {
    fn name(&self) -> &str {
        "build-annotations"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let cfg = Config::try_from(ctx)?;
        debug!("Config: {:?}", cfg);

        let cargo_file = std::fs::read_to_string(cfg.workspace_dir.join("Cargo.toml"))?;
        let cargo_toml: CargoToml = toml::from_str(&cargo_file)?;

        let commit = determine_git_rev(&cfg.git_dir, cfg.commit_characters);

        debug!(
            "Package: {} v{} Git commit: {}",
            cargo_toml.name().unwrap_or("unknown"),
            cargo_toml.version().unwrap_or("unknown"),
            commit.as_deref().unwrap_or("unknown")
        );

        let mut footer = String::new();

        if cfg.package_name {
            if let Some(name) = cargo_toml.name() {
                footer.push_str(name);
            } else {
                error!("Package name not found in Cargo.toml, skipping it in annotation");
            }
        }
        if cfg.git_commit {
            if let Some(commit) = &commit {
                if !footer.is_empty() {
                    footer.push(' ');
                }
                footer.push_str(&format!("@{}", commit));
            } else {
                error!("Git commit not found, skipping it in annotation");
            }
        }
        if cfg.package_version {
            if let Some(version) = cargo_toml.version() {
                if !footer.is_empty() {
                    footer.push(' ');
                }
                footer.push_str(&format!("v{}", version));
            } else {
                error!("Package version not found in Cargo.toml, skipping it in annotation");
            }
        }

        if footer.is_empty() {
            error!("No annotation data found, not adding footer");
            return Ok(book);
        }
        footer = format!("<footer>{footer}</footer>");

        book.for_each_mut(|item| self.handle_bookitem(item, &footer));

        Ok(book)
    }
}

fn determine_git_rev(workspace_dir: &PathBuf, commit_characters: usize) -> Option<String> {
    debug!(
        "looking for git repository in {}",
        workspace_dir.canonicalize().ok()?.display()
    );
    let Ok(repo) = gix::open(workspace_dir) else {
        error!("Failed to open git repository, can't annotate it!");
        return None;
    };

    let mut head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let mut commit_id = commit.id().to_string();
    // Now we actually want to trim this to the first `commit_characters` chars
    commit_id.truncate(commit_characters);
    Some(commit_id)
}

pub fn handle_preprocessing() -> Result<(), Error> {
    let (ctx, book) = parse_input(std::io::stdin())?;

    if ctx.mdbook_version != mdbook_preprocessor::MDBOOK_VERSION {
        warn!(
            "Warning: The {} preprocessor was built against version \
             {} of mdbook, but we're being called from version {}",
            Processor.name(),
            mdbook_preprocessor::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = Processor.run(&ctx, book)?;
    serde_json::to_writer(std::io::stdout(), &processed_book)?;

    Ok(())
}

pub fn handle_supports(proc: impl Preprocessor, renderer: &str) -> ! {
    let supported = proc.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if let Ok(true) = supported {
        exit(0);
    } else {
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_rev() {
        let rev = determine_git_rev(&env!("CARGO_MANIFEST_DIR").into(), 10);
        assert!(rev.is_some());
        assert_eq!(rev.as_ref().unwrap().len(), 10);
    }

    #[test]
    fn test_git_rev_too_long() {
        let rev = determine_git_rev(&env!("CARGO_MANIFEST_DIR").into(), 100);
        assert!(rev.is_some());
        assert_eq!(rev.as_ref().unwrap().len(), 40);
    }

    #[test]
    fn test_cargo_toml() {
        let cargo_file =
            std::fs::read_to_string(env!("CARGO_MANIFEST_DIR").to_string() + "/Cargo.toml")
                .expect("Failed to read Cargo.toml");
        let cargo_toml: CargoToml =
            toml::from_str(&cargo_file).expect("Failed to parse Cargo.toml");

        assert_eq!(
            cargo_toml.name().expect("Package name not found"),
            env!("CARGO_PKG_NAME")
        );
        assert_eq!(
            cargo_toml.version().expect("Package version not found"),
            env!("CARGO_PKG_VERSION")
        );
    }
}
