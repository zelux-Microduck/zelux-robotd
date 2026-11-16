//! Regression tests for the real atomic writer, no device required.
use crate::{BusProtocol, ImuModel, Params, edit, imu_management};

#[test]
fn selecting_model_preserves_other_settings_and_rejects_stale_revision() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    let original = r#"# retained
[bus]
protocol = "feetech-sts"
[bus.imu]
source = "ipc"
max_age_ms = 123
"#;
    std::fs::write(&file, original).unwrap();
    let params = Params::load(&file, true).unwrap();
    let revision = imu_management::revision(params.bus.protocol, &params.bus.imu);
    assert!(edit::select_imu_model(&file, ImuModel::QingmuB1, &revision).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    edit::select_imu_model(&file, ImuModel::QingmuA1, &revision).unwrap();
    let selected = Params::load(&file, true).unwrap();
    assert_eq!(selected.bus.protocol, BusProtocol::FeetechSts);
    assert_eq!(selected.bus.imu.model, Some(ImuModel::QingmuA1));
    assert_eq!(selected.bus.imu.max_age_ms, 123);
    assert!(selected.bus.imu.mount_wxyz.is_none());
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("# retained")
    );
    let before = std::fs::read_to_string(&file).unwrap();
    assert!(edit::select_imu_model(&file, ImuModel::QingmuA1, &revision).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
}

#[test]
fn selecting_does_not_silently_switch_servo_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    std::fs::write(
        &file,
        "# defaults
",
    )
    .unwrap();
    let params = Params::load(&file, true).unwrap();
    let revision = imu_management::revision(params.bus.protocol, &params.bus.imu);
    assert!(edit::select_imu_model(&file, ImuModel::QingmuA1, &revision).is_err());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# defaults
"
    );
}

#[test]
fn calibration_write_is_revision_guarded_and_model_selection_clears_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    std::fs::write(&file,
        "[bus]\nprotocol = \"feetech-sts\"\n[bus.imu]\nsource = \"imu-dxl\"\nmodel = \"qingmu-a1\"\n").unwrap();
    let params = Params::load(&file, true).unwrap();
    let revision = imu_management::revision(params.bus.protocol, &params.bus.imu);
    let correction = [std::f64::consts::FRAC_1_SQRT_2, 0.0, 0.0,
        std::f64::consts::FRAC_1_SQRT_2];
    edit::set_imu_mount_correction(&file, correction, &revision).unwrap();
    let calibrated = Params::load(&file, true).unwrap();
    assert_eq!(calibrated.bus.imu.mount_correction_wxyz, Some(correction));
    let stale_text = std::fs::read_to_string(&file).unwrap();
    assert!(edit::set_imu_mount_correction(&file, [1.0, 0.0, 0.0, 0.0], &revision).is_err());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), stale_text);

    let next = imu_management::revision(calibrated.bus.protocol, &calibrated.bus.imu);
    edit::select_imu_model(&file, ImuModel::QingmuA1, &next).unwrap();
    assert_eq!(Params::load(&file, true).unwrap().bus.imu.mount_correction_wxyz, Some(correction));

    let calibrated = Params::load(&file, true).unwrap();
    let revision = imu_management::revision(calibrated.bus.protocol, &calibrated.bus.imu);
    edit::select_imu_model(&file, ImuModel::QingmuB1, &revision).unwrap_err();
    assert_eq!(Params::load(&file, true).unwrap().bus.imu.mount_correction_wxyz, Some(correction));
}
