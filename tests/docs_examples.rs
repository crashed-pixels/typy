//! Execute the complete projects embedded in the Markdown documentation.
use std::{collections::BTreeMap, fs, path::Path};
use typy::modules::{ModuleLoader, Program, SourceModule};

struct Sources(BTreeMap<String, String>);
impl ModuleLoader for Sources {
    fn load(&mut self, importer: &str, module: &str) -> Result<SourceModule, String> {
        let local = importer
            .rsplit_once('/')
            .map(|(parent, _)| format!("{parent}/"))
            .unwrap_or_default();
        let module = module.replace('.', "/");
        for prefix in [local.as_str(), ""] {
            for path in [
                format!("{prefix}{module}.tp"),
                format!("{prefix}{module}/lib.tp"),
            ] {
                if let Some(source) = self.0.get(&path) {
                    return Ok(SourceModule::new(path, source.clone()));
                }
            }
        }
        Err(format!("ImportError: {module}"))
    }
}
fn check_document(path: &Path) -> bool {
    let text = fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    let mut files = BTreeMap::new();
    let mut expected = None;
    while let Some(line) = lines.next() {
        if let Some(name) = line.strip_prefix("```typy ") {
            let mut source = String::new();
            for line in lines.by_ref().take_while(|line| *line != "```") {
                source.push_str(line);
                source.push('\n');
            }
            assert!(
                files.insert(name.to_string(), source).is_none(),
                "duplicate example {name}"
            );
        } else if line == "```output" {
            let mut output = String::new();
            for line in lines.by_ref().take_while(|line| *line != "```") {
                output.push_str(line);
                output.push('\n');
            }
            assert!(
                expected.replace(output).is_none(),
                "duplicate output in {}",
                path.display()
            );
        }
    }
    if files.is_empty() {
        return false;
    }
    let entry = files.remove("main.tp").expect("example needs main.tp");
    let program = Program::compile(SourceModule::new("main.tp", entry), &mut Sources(files))
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut output = String::new();
    program
        .run(|line| {
            output.push_str(line);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        output,
        expected.expect("example needs output"),
        "{}",
        path.display()
    );
    true
}
#[test]
fn documentation_projects_match_their_documented_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for entry in fs::read_dir(root.join("docs")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "md") && check_document(&path) {
            checked += 1;
        }
    }
    assert!(
        checked >= 6,
        "expected runnable examples covering the language, got {checked}"
    );
    assert!(check_document(&root.join("README.md")));
}
