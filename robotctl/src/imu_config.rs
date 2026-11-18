//! IMU management endpoint for Studio/Agent. JSON only, never starts robotd.
use std::path::Path;

pub fn restart(socket: &Path, expected: &str) -> Result<(), String> {
    let mut client = crate::Client::connect_to("robotd", socket).map_err(|e| e.message)?;
    client.hello().map_err(|e| e.message)?;
    let call = duck_ipc_proto::Call::RobotRestartImu(duck_ipc_proto::ImuRestartParams {
        expected: expected.to_owned(),
        confirmed: true,
    });
    let result =
        crate::result_of(client.call(&call).map_err(|e| e.message)?).map_err(|e| e.message)?;
    if result.get("accepted").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(result
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("robotd refused the restart request")
            .to_owned());
    }
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}

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
    socket: &Path,
    file: &Path,
    model: Option<&str>,
    mount_correction_wxyz: Option<&str>,
    expected: Option<&str>,
) -> Result<(), String> {
    if model.is_some() || mount_correction_wxyz.is_some() {
        let expected = expected.ok_or("--expected is required when saving IMU configuration")?;
        let correction = mount_correction_wxyz
            .map(|raw| {
                serde_json::from_str::<[f64; 4]>(raw).map_err(|e| {
                    format!("--mount-correction-wxyz must be a JSON array of four numbers: {e}")
                })
            })
            .transpose()?;
        let mut client = crate::Client::connect_to("robotd", socket).map_err(|e| e.message)?;
        client.hello().map_err(|e| e.message)?;
        let request = duck_ipc_proto::Call::RobotConfigureImu(duck_ipc_proto::ImuConfigParams {
            config_path: file.to_string_lossy().into_owned(),
            model: model.map(str::to_owned),
            mount_correction_wxyz: correction,
            expected: expected.to_owned(),
        });
        let result = crate::result_of(client.call(&request).map_err(|e| e.message)?)
            .map_err(|e| e.message)?;
        if result.get("accepted").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(result
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("robotd refused the IMU configuration change")
                .to_owned());
        }
        let management = result
            .get("management")
            .ok_or("robotd accepted the change but returned no configuration snapshot")?;
        println!(
            "{}",
            serde_json::to_string(management).map_err(|e| e.to_string())?
        );
        return Ok(());
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
