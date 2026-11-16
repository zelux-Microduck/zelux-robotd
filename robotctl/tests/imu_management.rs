//! CLI contract tests use temporary files only. Never connect to a bus or restart.
use std::process::{Command, Output};
fn invoke(file: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_robotctl"))
        .arg("imu-config")
        .arg("--file")
        .arg(file)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn json_catalog_and_guarded_selection_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    std::fs::write(&file, "[bus]\nprotocol = \"feetech-sts\"\n").unwrap();
    let report = invoke(&file, &[]);
    assert!(report.status.success(), "{:?}", report);
    let before: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(before["schema"], "zelux.robotd-imu-management/v1");
    let revision = before["revision"].as_str().unwrap();
    assert!(
        !invoke(&file, &["--model", "qingmu-b1", "--expected", revision])
            .status
            .success()
    );
    assert!(!invoke(&file, &["--model", "qingmu-a1"]).status.success());
    let selected = invoke(&file, &["--model", "qingmu-a1", "--expected", revision]);
    assert!(selected.status.success(), "{:?}", selected);
    let after: serde_json::Value = serde_json::from_slice(&selected.stdout).unwrap();
    assert_eq!(after["config"]["imu"]["model"], "qingmu-a1");
    assert_eq!(after["config"]["imu"]["source"], "imu-dxl");
    assert_ne!(before["revision"], after["revision"]);
    assert!(
        !invoke(&file, &["--model", "qingmu-a1", "--expected", revision])
            .status
            .success()
    );
}
