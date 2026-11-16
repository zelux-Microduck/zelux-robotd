//! Fixed hardware mounting profiles. No transport access and no control-policy branches.
use serde::{Deserialize, Serialize};

/// Physical model identity is distinct from the IMU-DXL wire decoder identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImuModel {
    QingmuA1,
    QingmuB1,
}

impl ImuModel {
    pub const fn id(self) -> &'static str {
        match self {
            Self::QingmuA1 => "qingmu-a1",
            Self::QingmuB1 => "qingmu-b1",
        }
    }

    /// Version of the fixed physical mounting definition, not the driver version.
    pub const fn mount_profile(self) -> &'static str {
        match self {
            Self::QingmuA1 => "qingmu-a1.vertical.v1",
            Self::QingmuB1 => "qingmu-b1.horizontal.v1",
        }
    }

    /// Whether this model's validated driver/parameter path supports body-alignment calibration.
    pub const fn calibration_supported(self) -> bool {
        matches!(self, Self::QingmuA1)
    }

    /// Sensor-to-body WXYZ, body axes forward/left/up. q and -q are equivalent.
    /// B1's known mounting does NOT establish wire-driver support.
    pub const fn mount_wxyz(self) -> [f64; 4] {
        match self {
            // Sensor +X down, +Y right, +Z back: body = [-sz, -sy, -sx].
            Self::QingmuA1 => [
                0.0,
                std::f64::consts::FRAC_1_SQRT_2,
                0.0,
                -std::f64::consts::FRAC_1_SQRT_2,
            ],
            // Sensor +X back, +Y right, +Z up: body = [-sx, -sy, sz].
            Self::QingmuB1 => [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImuModel;
    use crate::{BusImu, BusProtocol, ImuSourceKind, Params, ParamsError};

    fn a1() -> BusImu {
        BusImu {
            source: ImuSourceKind::ImuDxl,
            model: Some(ImuModel::QingmuA1),
            ..BusImu::default()
        }
    }

    #[test]
    fn model_names_round_trip_and_unknown_models_fail() {
        for (id, model) in [
            ("qingmu-a1", ImuModel::QingmuA1),
            ("qingmu-b1", ImuModel::QingmuB1),
        ] {
            let text = format!("[bus.imu]\nmodel = \"{id}\"\n");
            let p: Params = toml::from_str(&text).unwrap();
            assert_eq!(p.bus.imu.model, Some(model));
            let round_trip: Params = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
            assert_eq!(round_trip.bus.imu.model.unwrap().id(), id);
        }
        assert!(toml::from_str::<Params>("[bus.imu]\nmodel = \"qingmu-c1\"\n").is_err());
    }

    #[test]
    fn a1_resolves_without_explicit_mount_and_requires_feetech() {
        assert_eq!(
            a1().resolve_mount(BusProtocol::FeetechSts).unwrap(),
            ImuModel::QingmuA1.mount_wxyz()
        );
        assert!(a1().resolve_mount(BusProtocol::Dynamixel2).is_err());
        assert_eq!(ImuModel::QingmuA1.mount_profile(), "qingmu-a1.vertical.v1");
        assert_eq!(
            ImuModel::QingmuB1.mount_profile(),
            "qingmu-b1.horizontal.v1"
        );
    }

    #[test]
    fn equivalent_explicit_rotation_is_accepted_but_conflicts_fail() {
        let mut imu = a1();
        imu.mount_wxyz = Some(ImuModel::QingmuA1.mount_wxyz().map(|x| -2.0 * x));
        assert_eq!(
            imu.resolve_mount(BusProtocol::FeetechSts).unwrap(),
            ImuModel::QingmuA1.mount_wxyz()
        );
        imu.mount_wxyz = Some([1.0, 0.0, 0.0, 0.0]);
        assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
    }

    #[test]
    fn guided_correction_composes_after_the_model_mount() {
        let mut imu = a1();
        let correction = [std::f64::consts::FRAC_1_SQRT_2, 0.0, 0.0,
            std::f64::consts::FRAC_1_SQRT_2];
        imu.mount_correction_wxyz = Some(correction);
        let resolved = imu.resolve_mount(BusProtocol::FeetechSts).unwrap();
        let base = ImuModel::QingmuA1.mount_wxyz();
        let expected = [
            correction[0] * base[0] - correction[1] * base[1] - correction[2] * base[2] - correction[3] * base[3],
            correction[0] * base[1] + correction[1] * base[0] + correction[2] * base[3] - correction[3] * base[2],
            correction[0] * base[2] - correction[1] * base[3] + correction[2] * base[0] + correction[3] * base[1],
            correction[0] * base[3] + correction[1] * base[2] - correction[2] * base[1] + correction[3] * base[0],
        ];
        for (got, want) in resolved.into_iter().zip(expected) {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn b1_is_known_but_cannot_activate_unverified_wire_driver() {
        let mut imu = a1();
        imu.model = Some(ImuModel::QingmuB1);
        imu.mount_wxyz = Some(ImuModel::QingmuB1.mount_wxyz());
        for protocol in [BusProtocol::FeetechSts, BusProtocol::Dynamixel2] {
            assert!(
                imu.resolve_mount(protocol)
                    .unwrap_err()
                    .contains("not verified")
            );
        }
    }

    #[test]
    fn legacy_explicit_mount_remains_supported_without_model() {
        let mut imu = BusImu {
            source: ImuSourceKind::ImuDxl,
            ..BusImu::default()
        };
        assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        imu.mount_wxyz = Some([2.0, 0.0, 0.0, 0.0]);
        for protocol in [BusProtocol::FeetechSts, BusProtocol::Dynamixel2] {
            assert_eq!(imu.resolve_mount(protocol).unwrap(), [1.0, 0.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn invalid_mount_timeout_and_source_are_rejected() {
        for q in [[0.0; 4], [f64::NAN; 4], [f64::INFINITY; 4]] {
            let mut imu = a1();
            imu.mount_wxyz = Some(q);
            assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        }
        let mut imu = a1();
        imu.mount_correction_wxyz = Some([0.0; 4]);
        assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        let mut imu = a1();
        imu.mount_correction_wxyz = Some([f64::NAN, 0.0, 0.0, 1.0]);
        assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        let mut imu = a1();
        imu.max_age_ms = 0;
        assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        for source in [
            ImuSourceKind::Ipc,
            ImuSourceKind::Embedded,
            ImuSourceKind::Unavailable,
        ] {
            let mut imu = a1();
            imu.source = source;
            assert!(imu.resolve_mount(BusProtocol::FeetechSts).is_err());
        }
    }

    #[test]
    fn file_loader_enforces_model_rules_before_startup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("robotd.toml");
        let valid = "[bus]\nprotocol = \"feetech-sts\"\n[bus.imu]\nsource = \"imu-dxl\"\nmodel = \"qingmu-a1\"\n";
        std::fs::write(&path, valid).unwrap();
        assert!(Params::load(&path, true).is_ok());
        for invalid in [
            valid.replace("qingmu-a1", "qingmu-b1"),
            valid.replace("feetech-sts", "dynamixel2"),
            valid.replace("imu-dxl", "ipc"),
            format!("{valid}mount_wxyz = [1.0, 0.0, 0.0, 0.0]\n"),
        ] {
            std::fs::write(&path, invalid).unwrap();
            assert!(matches!(
                Params::load(&path, true),
                Err(ParamsError::Imu { .. })
            ));
        }
    }
}
