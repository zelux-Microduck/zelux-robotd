//! IMU source boundary for the real-time control loop.
//!
//! Motor transport and orientation are deliberately separate here.  A Dynamixel robot can
//! still obtain both in one transaction, while a Feetech robot can supply orientation from
//! I2C, SPI, shared memory, or another process without teaching the policy loop about any of
//! those transports.

use crate::imu::ImuData;
use crate::io::{ImuStale, JointTargets, Result, RobotIo, Sensors, SlowSensors};

/// One IMU measurement plus the liveness information used by safety and health reporting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuSample {
    pub data: ImuData,
    pub ready: bool,
    pub stale: ImuStale,
}

impl Default for ImuSample {
    fn default() -> Self {
        Self {
            data: ImuData::default(),
            ready: false,
            stale: ImuStale::default(),
        }
    }
}

/// A replaceable producer of orientation samples.
///
/// `embedded` is the sample returned by the motor backend.  Sources backed by a combined
/// servo/IMU transaction use it without another hardware read; independent sources ignore it.
pub trait ImuSource {
    fn id(&self) -> &'static str;
    fn sample(&mut self, embedded: Option<ImuSample>) -> Result<ImuSample>;
}

/// Compatibility source for the original `imu_to_dxl` layout.
///
/// This keeps the existing single bus transaction and therefore changes no real-time behaviour.
#[derive(Debug, Default)]
pub struct EmbeddedImuSource;

impl ImuSource for EmbeddedImuSource {
    fn id(&self) -> &'static str {
        "embedded"
    }

    fn sample(&mut self, embedded: Option<ImuSample>) -> Result<ImuSample> {
        Ok(embedded.unwrap_or_default())
    }
}

/// Explicitly unavailable source used during hardware bring-up.
///
/// It is intentionally not "fake upright": safety sees `ready=false` and refuses to drive.
#[derive(Debug, Default)]
pub struct UnavailableImuSource;

impl ImuSource for UnavailableImuSource {
    fn id(&self) -> &'static str {
        "unavailable"
    }

    fn sample(&mut self, _embedded: Option<ImuSample>) -> Result<ImuSample> {
        Ok(ImuSample::default())
    }
}

/// Runtime-selected built-in source. External adapters can add variants without changing the
/// control loop or motor backends.
pub enum SelectedImuSource {
    Embedded(EmbeddedImuSource),
    Unavailable(UnavailableImuSource),
}

impl SelectedImuSource {
    pub fn embedded() -> Self {
        Self::Embedded(EmbeddedImuSource)
    }

    pub fn unavailable() -> Self {
        Self::Unavailable(UnavailableImuSource)
    }
}

impl ImuSource for SelectedImuSource {
    fn id(&self) -> &'static str {
        match self {
            Self::Embedded(source) => source.id(),
            Self::Unavailable(source) => source.id(),
        }
    }

    fn sample(&mut self, embedded: Option<ImuSample>) -> Result<ImuSample> {
        match self {
            Self::Embedded(source) => source.sample(embedded),
            Self::Unavailable(source) => source.sample(embedded),
        }
    }
}

/// Combines a motor backend with an independently selectable IMU source.
///
/// All motor operations remain owned by `B`.  Only the IMU fields in the atomic sensor sample
/// are supplied by `S`, which is the seam used by future I2C and IPC adapters.
pub struct SplitImuIo<B, S> {
    bus: B,
    source: S,
    last_imu: ImuSample,
}

impl<B, S> SplitImuIo<B, S> {
    pub fn new(bus: B, source: S) -> Self {
        Self {
            bus,
            source,
            last_imu: ImuSample::default(),
        }
    }

    pub fn bus(&self) -> &B {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut B {
        &mut self.bus
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }
}

impl<B: RobotIo, S: ImuSource> RobotIo for SplitImuIo<B, S> {
    fn read(&mut self) -> Result<Sensors> {
        let mut sensors = self.bus.read()?;
        let embedded = ImuSample {
            data: sensors.imu,
            ready: self.bus.imu_ready(),
            stale: self.bus.imu_stale(),
        };
        self.last_imu = self.source.sample(Some(embedded))?;
        sensors.imu = self.last_imu.data;
        Ok(sensors)
    }

    fn write(&mut self, targets: &JointTargets) -> Result<()> {
        self.bus.write(targets)
    }

    fn set_gain(&mut self, kp: u16) -> Result<()> {
        self.bus.set_gain(kp)
    }

    fn set_torque(&mut self, on: bool) -> Result<()> {
        self.bus.set_torque(on)
    }

    fn reboot(&mut self, id: u8) -> Result<()> {
        self.bus.reboot(id)
    }

    fn slow_sensors(&mut self) -> Result<SlowSensors> {
        self.bus.slow_sensors()
    }

    fn imu_stale(&self) -> ImuStale {
        self.last_imu.stale
    }

    fn imu_ready(&self) -> bool {
        self.last_imu.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::FakeIo;

    struct FixedSource(ImuSample);

    impl ImuSource for FixedSource {
        fn id(&self) -> &'static str {
            "test.fixed"
        }

        fn sample(&mut self, _embedded: Option<ImuSample>) -> Result<ImuSample> {
            Ok(self.0)
        }
    }

    #[test]
    fn external_source_replaces_only_the_imu_part_of_the_sample() {
        let expected = ImuSample {
            data: ImuData {
                gyro: [1.0, 2.0, 3.0],
                gravity: [0.0, -1.0, 0.0],
                quat: [0.5, 0.5, 0.5, 0.5],
            },
            ready: true,
            stale: ImuStale { total: 7, run: 2 },
        };
        let mut io = SplitImuIo::new(FakeIo::default(), FixedSource(expected));

        let sensors = io.read().unwrap();

        assert_eq!(sensors.imu, expected.data);
        assert!(io.imu_ready());
        assert_eq!(io.imu_stale(), expected.stale);
    }

    #[test]
    fn embedded_source_preserves_the_combined_bus_behaviour() {
        let mut bus = FakeIo::default();
        let expected = ImuData {
            gyro: [0.1, 0.2, 0.3],
            ..ImuData::default()
        };
        bus.set_imu(expected);
        bus.imu_ready = true;
        let mut io = SplitImuIo::new(bus, EmbeddedImuSource);

        assert_eq!(io.read().unwrap().imu, expected);
        assert!(io.imu_ready());
    }

    #[test]
    fn unavailable_source_cannot_claim_the_robot_is_upright() {
        let mut bus = FakeIo::default();
        bus.imu_ready = true;
        let mut io = SplitImuIo::new(bus, SelectedImuSource::unavailable());

        let _ = io.read().unwrap();

        assert!(!io.imu_ready());
    }
}
