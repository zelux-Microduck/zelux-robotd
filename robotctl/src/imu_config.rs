//! IMU management endpoint for Studio/Agent. JSON only, never starts robotd.
use std::path::Path;
pub fn status(socket: &Path) -> Result<(), crate::Failure> {
    let mut client = crate::Client::connect_to("robotd", socket)?;
    let health: duck_ipc_proto::HealthResult = client
        .call(&duck_ipc_proto::Call::RobotHealth)?
        .result_as()
        .map_err(|e| crate::Failure::new(crate::exit::FAILED, e.to_string()))?;
    let restart: duck_ipc_proto::SafeToRestartResult = client
        .call(&duck_ipc_proto::Call::RobotSafeToRestart)?
        .result_as()
        .map_err(|e| crate::Failure::new(crate::exit::FAILED, e.to_string()))?;
    println!(
        "{}",
        serde_json::json!({"schema": "zelux.robotd-imu-status/v1",
        "health": health, "restart": restart})
    );
    Ok(())
}

pub fn run(
    file: &Path,
    model: Option<&str>,
    mount_correction_wxyz: Option<&str>,
    expected: Option<&str>,
) -> Result<(), String> {
    if let Some(model) = model {
        let model = match model {
            "qingmu-a1" => robotd_params::ImuModel::QingmuA1,
            "qingmu-b1" => robotd_params::ImuModel::QingmuB1,
            _ => return Err("unknown IMU model".into()),
        };
        robotd_params::edit::select_imu_model(
            file,
            model,
            expected.ok_or("--expected is required when selecting a model")?,
        )?;
    }
    if let Some(raw) = mount_correction_wxyz {
        let correction: [f64; 4] = serde_json::from_str(raw)
            .map_err(|e| format!("--mount-correction-wxyz must be a JSON array of four numbers: {e}"))?;
        robotd_params::edit::set_imu_mount_correction(
            file,
            correction,
            expected.ok_or("--expected is required when saving calibration")?,
        )?;
    }
    let params = robotd_params::Params::load(file, false).map_err(|e| e.to_string())?;
    let mut report = robotd_params::imu_management::snapshot(&params);
    report["config_path"] =
        serde_json::json!(std::fs::canonicalize(file).unwrap_or_else(|_| file.to_owned()));
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
