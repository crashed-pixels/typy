//! Filesystem policy for the portable compiler's ModuleLoader interface.
use std::{
    fs,
    path::{Path, PathBuf},
};
use typy::modules::{ModuleLoader, SourceModule};

pub(super) struct FileLoader {
    root: PathBuf,
}
impl FileLoader {
    pub(super) fn entry(path: &str) -> Result<(Self, SourceModule), String> {
        let path =
            fs::canonicalize(path).map_err(|error| format!("ImportError: {path}: {error}"))?;
        if path.file_name().is_none_or(|name| name != "main.tp") {
            return Err("EntryError: only main.tp can be executed; libraries use lib.tp".into());
        }
        let root = path
            .parent()
            .ok_or("ImportError: entry has no parent directory")?
            .to_path_buf();
        let loader = Self { root };
        let source = loader.read(&path)?;
        Ok((loader, source))
    }
    fn read(&self, path: &Path) -> Result<SourceModule, String> {
        let canonical = fs::canonicalize(path)
            .map_err(|error| format!("ImportError: {}: {error}", path.display()))?;
        if !canonical.starts_with(&self.root) {
            return Err("ImportError: module resolves outside the project root".into());
        }
        let source = fs::read_to_string(&canonical)
            .map_err(|error| format!("ImportError: {}: {error}", canonical.display()))?;
        Ok(SourceModule::new(
            canonical
                .to_str()
                .ok_or("ImportError: module paths must be valid UTF-8")?,
            source,
        ))
    }
}
impl ModuleLoader for FileLoader {
    fn load(&mut self, importer: &str, module: &str) -> Result<SourceModule, String> {
        let mut relative = PathBuf::new();
        for component in module.split('.') {
            if !valid_component(component) {
                return Err(format!("ImportError: invalid module name '{module}'"));
            }
            relative.push(component);
        }
        let local = Path::new(importer)
            .parent()
            .ok_or("ImportError: module has no parent directory")?;
        for base in [local, &self.root] {
            let module_path = base.join(&relative);
            let file = module_path.with_extension("tp");
            let library = module_path.join("lib.tp");
            match (file.is_file(), library.is_file()) {
                (true, true) => {
                    return Err(format!(
                        "ImportError: ambiguous module '{module}': both {} and {} exist",
                        file.display(),
                        library.display()
                    ));
                }
                (true, false) => return self.read(&file),
                (false, true) => return self.read(&library),
                (false, false) => {}
            }
            if local == self.root {
                break;
            }
        }
        Err(format!(
            "ImportError: module '{module}' not found; expected a .tp file or a directory with lib.tp"
        ))
    }
}
fn valid_component(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase() || ch == '_')
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}
