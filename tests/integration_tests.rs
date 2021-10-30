use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use tempfile::TempDir;

use ruplacer::Query;
use ruplacer::Settings;
use ruplacer::{DirectoryPatcher, Stats};

fn setup_test(tmp_dir: &TempDir) -> PathBuf {
    let tmp_path = tmp_dir.path();
    #[cfg(not(target_os = "windows"))]
    let status = Command::new("cp")
        .args(&["-R", "tests/data", &tmp_path.to_string_lossy()])
        .status()
        .expect("Failed to execute process");
    #[cfg(target_os = "windows")]
    let status = Command::new("xcopy")
        .args(&[
            "/E",
            "/I",
            "tests\\data",
            &tmp_path.join("data").to_string_lossy(),
        ])
        .status()
        .expect("Failed to execute process");
    assert!(status.success());
    tmp_path.join("data")
}

fn assert_replaced(path: &Path) {
    let contents =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Could not read from {:?}", path));
    assert!(contents.contains("new"));
    assert!(!contents.contains("old"));
}

fn assert_not_replaced(path: &Path) {
    let contents =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("Could not read from {:?}", path));
    assert!(!contents.contains("new"));
    assert!(contents.contains("old"));
}

fn run_ruplacer(data_path: &Path, settings: Settings) -> Result<Stats> {
    let mut directory_patcher = DirectoryPatcher::new(data_path, &settings);
    directory_patcher.run(&Query::substring("old", "new"))?;
    Ok(directory_patcher.stats())
}

fn temp_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("test-ruplacer")
        .tempdir()
        .unwrap()
}

#[test]
fn test_replace_old_by_new() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings::default();
    run_ruplacer(&data_path, settings).unwrap();
    let top_txt_path = data_path.join("top.txt");
    assert_replaced(&top_txt_path);

    // Also check recursion inside the data dir:
    let foo_path = data_path.join("a_dir/sub/foo.txt");
    assert_replaced(&foo_path);
}

#[test]
fn test_stats() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings::default();
    let stats = run_ruplacer(&data_path, settings).unwrap();
    assert!(stats.matching_files() > 1);
    assert!(stats.total_replacements() > 1);
}

#[test]
fn test_dry_run() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings {
        dry_run: true,
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    let top_txt_path = data_path.join("top.txt");
    assert_not_replaced(&top_txt_path);
}

#[test]
fn test_skip_hidden_and_ignored_by_default() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings::default();
    run_ruplacer(&data_path, settings).unwrap();

    let hidden_path = data_path.join(".hidden.txt");
    assert_not_replaced(&hidden_path);

    let ignored_path = data_path.join("ignore.txt");
    assert_not_replaced(&ignored_path);
}

#[test]
fn test_can_replace_hidden_files() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings {
        hidden: true,
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    let hidden_path = data_path.join(".hidden.txt");
    assert_replaced(&hidden_path);
}

#[test]
fn test_can_replace_ignored_files() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings {
        ignored: true,
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    let ignored_path = data_path.join("ignore.txt");
    assert_replaced(&ignored_path);
}

#[test]
fn test_skip_non_utf8_files() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    let bin_path = data_path.join("foo.latin1");
    fs::write(bin_path, b"caf\xef\n").unwrap();

    let settings = Settings::default();
    run_ruplacer(&data_path, settings).unwrap();
}

fn add_python_file(data_path: &Path) -> PathBuf {
    let py_path = data_path.join("foo.py");
    fs::write(&py_path, "a = 'this is old'\n").unwrap();
    py_path
}

#[test]
fn test_select_file_types() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    add_python_file(&data_path);

    let settings = Settings {
        selected_file_types: vec!["py".to_string()],
        ..Default::default()
    };
    let stats = run_ruplacer(&data_path, settings).unwrap();

    assert_eq!(stats.matching_files(), 1);
}

#[test]
fn test_select_file_types_by_glob_pattern_1() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    add_python_file(&data_path);

    let settings = Settings {
        selected_file_types: vec!["*.py".to_string()],
        ..Default::default()
    };
    let stats = run_ruplacer(&data_path, settings).unwrap();

    assert_eq!(stats.matching_files(), 1);
}

#[test]
fn test_select_file_types_by_glob_pattern_2() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    add_python_file(&data_path);

    let settings = Settings {
        selected_file_types: vec!["f*.py".to_string()],
        ..Default::default()
    };
    let stats = run_ruplacer(&data_path, settings).unwrap();

    assert_eq!(stats.matching_files(), 1);
}

#[test]
fn test_select_file_types_by_incorrect_glob_pattern() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);

    let settings = Settings {
        selected_file_types: vec!["[*.py".to_string()],
        ..Default::default()
    };
    let err = run_ruplacer(&data_path, settings).unwrap_err();
    assert!(err.to_string().contains("error parsing glob"));
}

#[test]
fn test_ignore_file_types() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    let py_path = add_python_file(&data_path);
    let settings = Settings {
        ignored_file_types: vec!["py".to_string()],
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    assert_not_replaced(&py_path);
}

#[test]
fn test_ignore_file_types_by_glob_pattern_1() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    let py_path = add_python_file(&data_path);
    let settings = Settings {
        ignored_file_types: vec!["*.py".to_string()],
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    assert_not_replaced(&py_path);
}

#[test]
fn test_ignore_file_types_by_glob_pattern_2() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    let py_path = add_python_file(&data_path);
    let settings = Settings {
        ignored_file_types: vec!["f*.py".to_string()],
        ..Default::default()
    };
    run_ruplacer(&data_path, settings).unwrap();

    assert_not_replaced(&py_path);
}

#[test]
fn test_ignore_file_types_by_incorrect_glob_pattern() {
    let tmp_dir = temp_dir();
    let data_path = setup_test(&tmp_dir);
    let settings = Settings {
        ignored_file_types: vec!["[.py".to_string()],
        ..Default::default()
    };
    let err = run_ruplacer(&data_path, settings).unwrap_err();
    assert!(err.to_string().contains("unrecognized file type"));
}
