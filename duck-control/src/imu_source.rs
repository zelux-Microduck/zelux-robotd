//! IMU source boundary for the real-time control loop.
//!
//! Motor transport and orientation are deliberately separate here.  A Dynamixel robot can
//! still obtain both in one transaction, while a Feetech robot can supply orientation from
//! I2C, SPI, shared memory, or another process without teaching the policy loop about any of
//! those transports.

use crate::imu::ImuData;
use crate::io::{ImuStale, IoError, JointTargets, Result, RobotIo, Sensors, SlowSensors};
#[cfg(unix)]
use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::time::{Duration, Instant};

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
    #[cfg(unix)]
    Ipc(IpcImuSource),
}

impl SelectedImuSource {
    pub fn embedded() -> Self {
        Self::Embedded(EmbeddedImuSource)
    }

    pub fn unavailable() -> Self {
        Self::Unavailable(UnavailableImuSource)
    }

    #[cfg(unix)]
    pub fn ipc(source: IpcImuSource) -> Self {
        Self::Ipc(source)
    }
}

impl ImuSource for SelectedImuSource {
    fn id(&self) -> &'static str {
        match self {
            Self::Embedded(source) => source.id(),
            Self::Unavailable(source) => source.id(),
            #[cfg(unix)]
            Self::Ipc(source) => source.id(),
        }
    }

    fn sample(&mut self, embedded: Option<ImuSample>) -> Result<ImuSample> {
        match self {
            Self::Embedded(source) => source.sample(embedded),
            Self::Unavailable(source) => source.sample(embedded),
            #[cfg(unix)]
            Self::Ipc(source) => source.sample(embedded),
        }
    }
}

/// Wire representation accepted from an external IMU driver process.
///
/// It is intentionally the measurement subset of `zelux.robotd-imu/v1`: driver identity and
/// publication metadata belong to the service boundary, while the 50 Hz loop only needs these
/// values. Unknown fields are accepted so additive protocol upgrades do not stop motion.
#[cfg(unix)]
#[derive(Deserialize)]
struct IpcPacket {
    schema: String,
    sequence: u64,
    gyro_rad_s: [f64; 3],
    gravity: [f64; 3],
    quaternion_wxyz: [f64; 4],
    ready: bool,
}

/// Non-blocking Unix datagram input for an IMU owned by another process.
///
/// Every control tick drains the socket and keeps only the newest sequence. Missing packets hold
/// the last numeric value for observability, but `ready` becomes false after `max_age`; safety can
/// therefore never mistake a dead producer for a stationary upright robot.
#[cfg(unix)]
pub struct IpcImuSource {
    socket: UnixDatagram,
    max_age: Duration,
    last: ImuSample,
    last_sequence: Option<u64>,
    last_received: Option<Instant>,
}

#[cfg(unix)]
impl IpcImuSource {
    pub fn bind(path: impl AsRef<Path>, max_age: Duration) -> Result<Self> {
        let path = path.as_ref();
        let socket = UnixDatagram::bind(path).map_err(|source| IoError::Port {
            path: path.display().to_string(),
            source,
        })?;
        socket
            .set_nonblocking(true)
            .map_err(|source| IoError::Port {
                path: path.display().to_string(),
                source,
            })?;
        Ok(Self {
            socket,
            max_age,
            last: ImuSample::default(),
            last_sequence: None,
            last_received: None,
        })
    }

    fn accept(&mut self, packet: IpcPacket) {
        if packet.schema != "zelux.robotd-imu/v1"
            || self
                .last_sequence
                .is_some_and(|sequence| packet.sequence <= sequence)
        {
            return;
        }
        self.last = ImuSample {
            data: ImuData {
                gyro: packet.gyro_rad_s,
                gravity: packet.gravity,
                quat: packet.quaternion_wxyz,
            },
            ready: packet.ready,
            stale: ImuStale {
                total: self.last.stale.total,
                run: 0,
            },
        };
        self.last_sequence = Some(packet.sequence);
        self.last_received = Some(Instant::now());
    }
}

#[cfg(unix)]
impl ImuSource for IpcImuSource {
    fn id(&self) -> &'static str {
        "ipc"
    }

    fn sample(&mut self, _embedded: Option<ImuSample>) -> Result<ImuSample> {
        let mut fresh = false;
        loop {
            let mut bytes = [0_u8; 2048];
            match self.socket.recv(&mut bytes) {
                Ok(len) => match serde_json::from_slice::<IpcPacket>(&bytes[..len]) {
                    Ok(packet) => {
                        let before = self.last_sequence;
                        self.accept(packet);
                        fresh |= self.last_sequence != before;
                    }
                    Err(error) => tracing::warn!(%error, "discarding malformed IMU datagram"),
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(IoError::Bus(format!("IMU IPC receive: {error}"))),
            }
        }

        if !fresh {
            self.last.stale.total = self.last.stale.total.saturating_add(1);
            self.last.stale.run = self.last.stale.run.saturating_add(1);
        }
        if self
            .last_received
            .is_none_or(|received| received.elapsed() > self.max_age)
        {
            self.last.ready = false;
        }
        Ok(self.last)
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

    #[cfg(unix)]
    #[test]
    fn ipc_source_accepts_new_samples_and_expires_them() {
        use std::os::unix::net::UnixDatagram;
        use std::time::{Duration, SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("robotd-imu-{}-{suffix}.sock", std::process::id()));
        let mut source = IpcImuSource::bind(&path, Duration::from_millis(20)).unwrap();
        let sender = UnixDatagram::unbound().unwrap();
        sender
            .send_to(
                br#"{"schema":"zelux.robotd-imu/v1","sequence":4,"gyro_rad_s":[1.0,2.0,3.0],"gravity":[0.0,0.0,-1.0],"quaternion_wxyz":[1.0,0.0,0.0,0.0],"ready":true}"#,
                &path,
            )
            .unwrap();

        let sample = source.sample(None).unwrap();
        assert_eq!(sample.data.gyro, [1.0, 2.0, 3.0]);
        assert!(sample.ready);

        std::thread::sleep(Duration::from_millis(25));
        assert!(!source.sample(None).unwrap().ready);
        std::fs::remove_file(path).unwrap();
    }
}
