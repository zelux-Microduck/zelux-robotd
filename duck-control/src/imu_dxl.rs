//! IMU-DXL model adapter. No serial access: the motor bus remains the sole owner.
//! Wire reference: IMU-DXL docs/PROTOCOL.md v1.2. This is NOT the old SFLP board.
use std::time::{Duration, Instant};

pub(crate) mod transaction;

use crate::imu::{ImuData, half, mul, rotate, rotate_inverse};
use crate::imu_source::ImuSample;
use crate::io::{IoError, Result};

pub const DEVICE_ID: u8 = 200;
pub const FEETECH_ADDRESS: u8 = 56;
pub const FEETECH_LENGTH: u8 = 15;
pub const DYNAMIXEL_ADDRESS: u8 = 124;
pub const DYNAMIXEL_LENGTH: u8 = 12;
pub const STATUS_ADDRESS: u8 = 143;
pub const DRIVER_ID: &str = "zelux.imu.imu-dxl";

/// Input has already passed the bus library framing/ID/checksum checks.
pub struct Frame {
    core: [u8; 12],
    status: u8,
    sequence: Option<u8>,
}

impl Frame {
    pub fn feetech(block: &[u8]) -> Result<Self> {
        if block.len() != FEETECH_LENGTH as usize {
            return Err(IoError::Bus("IMU-DXL: expected 15 Feetech bytes".into()));
        }
        Ok(Self {
            core: block[..12].try_into().expect("length checked"),
            status: block[13],
            sequence: Some(block[12]),
        })
    }

    pub fn dynamixel(core: &[u8], status: &[u8]) -> Result<Self> {
        if core.len() != DYNAMIXEL_LENGTH as usize || status.len() != 1 {
            return Err(IoError::Bus(
                "IMU-DXL: expected 12 core bytes and one status byte".into(),
            ));
        }
        Ok(Self {
            core: core.try_into().expect("length checked"),
            status: status[0],
            sequence: None,
        })
    }
}

/// A sensor-to-body mounting quaternion, explicitly configured and normalised.
/// Feetech freshness uses the hardware counter; Dynamixel exposes no counter in
/// this layout and can only establish successful-read freshness, not detect frozen fusion.
pub struct ImuDxl {
    mount: [f64; 4],
    max_age: Duration,
    last_sequence: Option<u8>,
    last_advance: Option<Instant>,
    sample: ImuSample,
}

impl ImuDxl {
    pub fn new(mount: [f64; 4], max_age: Duration) -> Result<Self> {
        let norm = mount.iter().map(|x| x * x).sum::<f64>().sqrt();
        if !norm.is_finite() || norm < 1e-6 || max_age.is_zero() {
            return Err(IoError::Bus(
                "IMU-DXL: invalid mounting quaternion or zero max_age".into(),
            ));
        }
        Ok(Self {
            mount: mount.map(|x| x / norm),
            max_age,
            last_sequence: None,
            last_advance: None,
            sample: ImuSample::default(),
        })
    }

    /// Reject a bad transaction immediately, retaining the last value only for diagnostics.
    pub fn observe(&mut self, frame: Result<Frame>, now: Instant) {
        let result = frame.and_then(|frame| self.accept(frame, now));
        if let Err(error) = result {
            if self.sample.ready
                || self.sample.stale.run == 0
                || self.sample.stale.run.is_multiple_of(500)
            {
                tracing::warn!(%error, "IMU-DXL not-ready");
            }
            self.sample.ready = false;
            self.sample.stale.total = self.sample.stale.total.saturating_add(1);
            self.sample.stale.run = self.sample.stale.run.saturating_add(1);
        }
    }

    fn accept(&mut self, frame: Frame, now: Instant) -> Result<()> {
        if frame.status & 1 != 0 {
            return Err(IoError::Bus("IMU-DXL: fusion not ready".into()));
        }
        let data = decode(&frame.core, self.mount)?;
        if let Some(sequence) = frame.sequence {
            if self.last_sequence == Some(sequence) {
                if self
                    .last_advance
                    .is_none_or(|at| now.duration_since(at) >= self.max_age)
                {
                    return Err(IoError::Bus("IMU-DXL: sensor sequence frozen".into()));
                }
                // Repeated reads within the hardware sampling period are not new samples.
                return Ok(());
            }
            // The counter wraps at 255 and may restart with the device. Any changed
            // value establishes a new arrival; it does not imply a measured sample timestamp.
            self.last_sequence = Some(sequence);
        }
        self.last_advance = Some(now);
        self.sample.data = data;
        self.sample.ready = true;
        self.sample.stale.run = 0;
        Ok(())
    }

    pub fn sample(&self, now: Instant) -> ImuSample {
        let mut sample = self.sample;
        sample.ready &= self
            .last_advance
            .is_some_and(|at| now.duration_since(at) < self.max_age);
        sample
    }
}

fn decode(core: &[u8; 12], mount: [f64; 4]) -> Result<ImuData> {
    let gyro = std::array::from_fn(|axis| {
        let i = axis * 2;
        i16::from_le_bytes([core[i], core[i + 1]]) as f64 / 16.4 * std::f64::consts::PI / 180.0
    });
    let xyz: [f64; 3] = std::array::from_fn(|axis| {
        let i = 6 + axis * 2;
        half(u16::from_le_bytes([core[i], core[i + 1]]))
    });
    let norm_sq = xyz.iter().map(|x| x * x).sum::<f64>();
    if !norm_sq.is_finite() || norm_sq > 1.02 {
        return Err(IoError::Bus("IMU-DXL: invalid quaternion".into()));
    }
    let sensor = [(1.0 - norm_sq).max(0.0).sqrt(), xyz[0], xyz[1], xyz[2]];
    let inverse_mount = [mount[0], -mount[1], -mount[2], -mount[3]];
    let quat = mul(sensor, inverse_mount);
    let norm = quat.iter().map(|x| x * x).sum::<f64>().sqrt();
    let quat = quat.map(|x| x / norm);
    Ok(ImuData {
        gyro: rotate(mount, gyro),
        gravity: rotate_inverse(quat, [0.0, 0.0, -1.0]),
        quat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

    #[test]
    fn fixed_profiles_map_all_signed_gyro_axes_to_body() {
        use robotd_params::ImuModel;
        let cases = [
            (
                ImuModel::QingmuA1,
                [[0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]],
            ),
            (
                ImuModel::QingmuB1,
                [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
            ),
        ];
        for (model, columns) in cases {
            for (axis, expected) in columns.into_iter().enumerate() {
                for sign in [-1i16, 1] {
                    let mut core = [0; 12];
                    core[axis * 2..axis * 2 + 2].copy_from_slice(&(sign * 164).to_le_bytes());
                    let data = decode(&core, model.mount_wxyz()).unwrap();
                    for (got, want) in data.gyro.into_iter().zip(expected) {
                        assert!((got - want * f64::from(sign) * 10f64.to_radians()).abs() < 1e-10);
                    }
                }
            }
        }
    }

    #[test]
    fn fixed_profiles_transform_quaternion_and_gravity_consistently() {
        use robotd_params::ImuModel;
        for (model, expected_gravity) in [
            (ImuModel::QingmuA1, [1.0, 0.0, 0.0]),
            (ImuModel::QingmuB1, [0.0, 0.0, -1.0]),
        ] {
            let mount = model.mount_wxyz();
            let data = decode(&[0; 12], mount).unwrap();
            let expected_quat = [mount[0], -mount[1], -mount[2], -mount[3]];
            let dot = data
                .quat
                .into_iter()
                .zip(expected_quat)
                .map(|(a, b)| a * b)
                .sum::<f64>();
            assert!((dot.abs() - 1.0).abs() < 1e-10);
            for (got, want) in data.gravity.into_iter().zip(expected_gravity) {
                assert!((got - want).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn a1_noncommuting_pose_uses_sensor_then_inverse_mount_order() {
        // Body roll +90 degrees, sensor = body * A1 mount, canonical W >= 0.
        // Exact binary16 XYZ = [-0.5, -0.5, 0.5].
        let mut core = [0; 12];
        for (axis, bits) in [0xb800u16, 0xb800, 0x3800].into_iter().enumerate() {
            core[6 + axis * 2..8 + axis * 2].copy_from_slice(&bits.to_le_bytes());
        }
        let data = decode(&core, robotd_params::ImuModel::QingmuA1.mount_wxyz()).unwrap();
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let expected = [q, q, 0.0, 0.0];
        let dot = data
            .quat
            .into_iter()
            .zip(expected)
            .map(|(a, b)| a * b)
            .sum::<f64>();
        assert!((dot.abs() - 1.0).abs() < 1e-10);
        for (got, want) in data.gravity.into_iter().zip([0.0, -1.0, 0.0]) {
            assert!((got - want).abs() < 1e-10);
        }
    }

    #[test]
    fn fixed_profiles_decode_level_pose_with_binary16_quantisation() {
        use robotd_params::ImuModel;
        // Encoded sensor-to-world = sensor-to-body for a level body.
        // A1 sqrt(1/2) rounds to 0x39a8; reconstructed W amplifies that error near pi.
        for (model, xyz) in [
            (ImuModel::QingmuA1, [0x39a8u16, 0, 0xb9a8]),
            (ImuModel::QingmuB1, [0, 0, 0x3c00]),
        ] {
            let mut core = [0; 12];
            for (axis, bits) in xyz.into_iter().enumerate() {
                core[6 + axis * 2..8 + axis * 2].copy_from_slice(&bits.to_le_bytes());
            }
            let data = decode(&core, model.mount_wxyz()).unwrap();
            assert!(data.quat[0].abs() > 0.999);
            for (got, want) in data.gravity.into_iter().zip([0.0, 0.0, -1.0]) {
                assert!((got - want).abs() < 0.05);
            }
        }
    }

    fn adapter() -> ImuDxl {
        ImuDxl::new(IDENTITY, Duration::from_millis(100)).unwrap()
    }
    fn frame(seq: u8) -> Frame {
        let mut block = [0; 15];
        block[12] = seq;
        Frame::feetech(&block).unwrap()
    }
    #[test]
    fn identity_is_a_valid_ready_pose_not_uninitialised_sflp() {
        let now = Instant::now();
        let mut imu = adapter();
        imu.observe(Ok(frame(0)), now);
        let sample = imu.sample(now);
        assert!(sample.ready);
        assert_eq!(sample.data.quat, IDENTITY);
        assert_eq!(sample.data.gravity, [0.0, 0.0, -1.0]);
    }
    #[test]
    fn signed_gyro_uses_imu_dxl_scale_not_legacy_scale() {
        let mut core = [0; 12];
        core[..2].copy_from_slice(&164i16.to_le_bytes());
        core[2..4].copy_from_slice(&(-164i16).to_le_bytes());
        let data = decode(&core, IDENTITY).unwrap();
        assert!((data.gyro[0] - 10f64.to_radians()).abs() < 1e-10);
        assert!((data.gyro[1] + 10f64.to_radians()).abs() < 1e-10);
    }
    #[test]
    fn counter_wrap_is_fresh_but_identical_counter_expires() {
        let now = Instant::now();
        let mut imu = adapter();
        imu.observe(Ok(frame(255)), now);
        imu.observe(Ok(frame(0)), now + Duration::from_millis(20));
        assert!(imu.sample(now + Duration::from_millis(20)).ready);
        imu.observe(Ok(frame(0)), now + Duration::from_millis(120));
        assert!(!imu.sample(now + Duration::from_millis(120)).ready);
        assert_eq!(imu.sample(now + Duration::from_millis(120)).stale.run, 1);
        imu.observe(Ok(frame(1)), now + Duration::from_millis(140));
        assert!(imu.sample(now + Duration::from_millis(140)).ready);
    }
    #[test]
    fn status_errors_and_bad_reads_inhibit_immediately() {
        let now = Instant::now();
        let mut imu = adapter();
        imu.observe(Ok(frame(1)), now);
        let mut not_ready = frame(2);
        not_ready.status = 1;
        imu.observe(Ok(not_ready), now);
        assert!(!imu.sample(now).ready);
        imu.observe(Ok(frame(3)), now);
        assert!(imu.sample(now).ready);
        imu.observe(Err(IoError::Simulated), now);
        assert!(!imu.sample(now).ready);
    }
    #[test]
    fn malformed_frames_and_nonfinite_quaternions_are_rejected() {
        assert!(Frame::feetech(&[0; 14]).is_err());
        assert!(Frame::dynamixel(&[0; 12], &[]).is_err());
        for bits in [0x7e00u16, 0x7c00, 0x4000] {
            let mut core = [0; 12];
            core[6..8].copy_from_slice(&bits.to_le_bytes());
            assert!(decode(&core, IDENTITY).is_err());
        }
    }
    #[test]
    fn dynamixel_validates_status_and_has_read_freshness_only() {
        let now = Instant::now();
        let mut imu = adapter();
        imu.observe(Frame::dynamixel(&[0; 12], &[0]), now);
        assert!(imu.sample(now).ready);
        assert!(!imu.sample(now + Duration::from_millis(101)).ready);
        imu.observe(Frame::dynamixel(&[0; 12], &[1]), now);
        assert!(!imu.sample(now).ready);
    }
    #[test]
    fn mounting_transforms_vectors_and_orientation_together() {
        let q = std::f64::consts::FRAC_1_SQRT_2;
        let mut imu = ImuDxl::new([q, 0.0, q, 0.0], Duration::from_millis(100)).unwrap();
        let mut block = [0; 15];
        block[..2].copy_from_slice(&164i16.to_le_bytes());
        let now = Instant::now();
        imu.observe(Frame::feetech(&block), now);
        let data = imu.sample(now).data;
        assert!(data.gyro[0].abs() < 1e-10);
        assert!((data.gyro[2] + 10f64.to_radians()).abs() < 1e-10);
        assert!((data.gravity[0] + 1.0).abs() < 1e-10);
    }
    #[test]
    fn failed_frame_does_not_refresh_counter_or_revive_on_duplicate() {
        let now = Instant::now();
        let mut imu = adapter();
        imu.observe(Ok(frame(1)), now);
        imu.observe(Err(IoError::Simulated), now + Duration::from_millis(20));
        imu.observe(Ok(frame(1)), now + Duration::from_millis(40));
        assert!(!imu.sample(now + Duration::from_millis(40)).ready);
        imu.observe(Ok(frame(2)), now + Duration::from_millis(60));
        assert!(imu.sample(now + Duration::from_millis(60)).ready);
    }

    #[test]
    fn quantized_unit_sphere_is_normalised_but_large_norm_is_rejected() {
        let mut core = [0; 12];
        core[6..8].copy_from_slice(&0x3c01u16.to_le_bytes());
        let data = decode(&core, IDENTITY).unwrap();
        let norm = data.quat.iter().map(|x| x * x).sum::<f64>();
        assert!((norm - 1.0).abs() < 1e-10);
        core[8..10].copy_from_slice(&0x3c00u16.to_le_bytes());
        assert!(decode(&core, IDENTITY).is_err());
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        assert!(ImuDxl::new([0.0; 4], Duration::from_millis(100)).is_err());
        assert!(ImuDxl::new([f64::NAN; 4], Duration::from_millis(100)).is_err());
        assert!(ImuDxl::new(IDENTITY, Duration::ZERO).is_err());
    }
}
