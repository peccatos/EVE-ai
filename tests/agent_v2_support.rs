#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

#[path = "evolution_test_support.rs"]
pub mod evolution_test_support;

pub fn temp_agent_root(name: &str) -> PathBuf {
    let root = evolution_test_support::unique_evolution_root(name);
    fs::create_dir_all(root.join("src/contracts")).expect("src contracts");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"agent_v2_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ndoctest = false\n",
    )
    .expect("cargo");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn stable_value() -> &'static str { \"stable\" }\n",
    )
    .expect("lib");
    fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    let _args = std::env::args().collect::<Vec<_>>();\n}\n",
    )
    .expect("main");
    fs::write(
        root.join("tests/basic_tests.rs"),
        "#[test]\nfn basic() { assert!(true); }\n",
    )
    .expect("test");
    fs::write(root.join("docs/intro.md"), "# Intro\n").expect("docs");
    fs::write(root.join("README.md"), "# Fixture\n").expect("readme");
    root
}
