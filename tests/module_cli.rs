#![cfg(feature = "cli")]
use std::{fs, path::PathBuf, process::{Command, Output}, sync::atomic::{AtomicUsize, Ordering}};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Project(PathBuf);
impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("typy_modules_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&root).unwrap();
        for (name, source) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        Self(root)
    }
    fn run(&self, path: &str, debug: bool) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_typy"));
        cmd.current_dir(&self.0).arg(self.0.join(path));
        if debug { cmd.arg("--debug"); }
        cmd.output().unwrap()
    }
}
impl Drop for Project { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
#[test]
fn binary_enters_main_and_print_is_the_only_stdout() {
    let p = Project::new(&[("main.tp", "def main() -> None:\n    999\n    print(42, True)\n")]);
    for debug in [false, true] {
        let output = p.run("main.tp", debug);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"42 True\n");
        if debug { assert!(!output.stderr.is_empty()); }
    }
}
#[test]
fn libraries_use_lib_tp_and_resolve_their_own_sibling_modules() {
    let p = Project::new(&[("main.tp", "from tools import twice\ndef main() -> None:\n    print(twice(21))\n"), ("tools/lib.tp", "from arithmetic import twice\n"), ("tools/arithmetic.tp", "def twice(n: int) -> int:\n    return n + n\n")]);
    let output = p.run("main.tp", false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.stdout, b"42\n");
}
#[test]
fn arbitrary_files_and_library_entries_cannot_be_executed() {
    let p = Project::new(&[("lib.tp", "def main() -> None:\n    print(1)\n"), ("script.tp", "print(2)\n"), ("main.tp", "print(3)\n")]);
    for path in ["lib.tp", "script.tp", "main.tp"] {
        let output = p.run(path, false);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
#[test]
fn ambiguous_module_and_library_paths_are_rejected() {
    let p = Project::new(&[("main.tp", "import tools\ndef main() -> None:\n    pass\n"), ("tools.tp", "x = 1\n"), ("tools/lib.tp", "x = 2\n")]);
    let output = p.run("main.tp", false);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ambiguous"));
}
