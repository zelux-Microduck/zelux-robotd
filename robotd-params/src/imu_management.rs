//! Machine-readable IMU management. No hardware access; robotd remains the bus owner.
use crate::{BusImu, BusProtocol, ImuModel, Params};
use serde_json::{Value, json};

/// Opaque concurrency token, not a cryptographic hash. Includes every IMU parameter
/// and the bus protocol; unrelated edits are preserved by the writer lock.
pub fn revision(protocol: BusProtocol, imu: &BusImu) -> String {
    serde_json::to_string(&json!({"protocol": protocol, "imu": imu})).expect("IMU serializes")
}

pub fn snapshot(params: &Params) -> Value {
    let models: Vec<Value> = [ImuModel::QingmuA1, ImuModel::QingmuB1]
        .into_iter().map(|model| {
            let candidate = BusImu { source: crate::ImuSourceKind::ImuDxl,
                model: Some(model), mount_wxyz: None, mount_correction_wxyz: None,
                ..params.bus.imu.clone() };
            let error = candidate.resolve_mount(params.bus.protocol).err();
            json!({"id": model.id(), "mount_profile": model.mount_profile(),
                "mount_wxyz": model.mount_wxyz(), "fixed_mount": true,
                "calibration_supported": model.calibration_supported(),
                "sensor_axes": match model { ImuModel::QingmuA1 => ["down", "right", "back"],
                    ImuModel::QingmuB1 => ["back", "right", "up"] },
                "supported": model == ImuModel::QingmuA1,
                "compatible": error.is_none(), "reason": error,
                "driver_id": if model == ImuModel::QingmuA1 { Some("zelux.imu.imu-dxl") } else { None },
                "driver_version": if model == ImuModel::QingmuA1 { Some(env!("CARGO_PKG_VERSION")) } else { None }})
        }).collect();
    json!({"schema": "zelux.robotd-imu-management/v1", "api_version": duck_ipc_proto::API_VERSION,
        "config": {"protocol": params.bus.protocol, "imu": params.bus.imu},
        "revision": revision(params.bus.protocol, &params.bus.imu), "models": models})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_never_enables_unverified_b1() {
        let mut p = Params::default();
        p.bus.protocol = BusProtocol::FeetechSts;
        let s = snapshot(&p);
        assert_eq!(s["models"][0]["compatible"], true);
        assert_eq!(s["models"][1]["compatible"], false);
        assert_eq!(s["models"][1]["supported"], false);
        p.bus.protocol = BusProtocol::Dynamixel2;
        assert_eq!(snapshot(&p)["models"][0]["compatible"], false);
    }
}
