//! Bench-only, bounded read-only diagnosis. Never starts robotd or writes registers.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use duck_control::imu_dxl::{Frame, ImuDxl};
use rustypot::servo::feetech::sts3215::Sts3215Controller;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mixed = args.len() == 3 && args[2] == "--mixed-feedback";
    if args.len() != 2 && !mixed {
        return Err("usage: imu_readonly /dev/ttyS2 [--mixed-feedback] (exclusive bench access required)".into());
    }
    let port = serialport::new(&args[1], 1_000_000)
        .timeout(Duration::from_millis(30))
        .open()?;
    let mut controller = Sts3215Controller::new()
        .with_protocol_v1()
        .with_serial_port(port);
    // STS torque-enable register 40. Missing/ambiguous readback is NOT torque-off.
    for id in duck_control::JOINT_IDS {
        let blocks = controller.sync_read_raw_data(&[id], 40, 1)?;
        if blocks.len() != 1 || blocks[0].as_slice() != [0] {
            return Err(format!("servo {id}: torque not confirmed off: {blocks:?}").into());
        }
        println!("{}", serde_json::json!({"servo_id":id,"torque_enable":0}));
    }
    let mount = robotd_params::ImuModel::QingmuA1.mount_wxyz();
    let mut imu = ImuDxl::new(mount, Duration::from_millis(100))?;
    let start = Instant::now();
    let count = if mixed { 1500 } else { 200 };
    let period = Duration::from_millis(if mixed { 20 } else { 50 });
    for index in 0..count {
        let tick = Instant::now();
        let feedback = if mixed {
            let blocks = controller.sync_read_raw_data(&duck_control::JOINT_IDS, 56, 15)?;
            if blocks.len() != duck_control::JOINT_IDS.len()
                || blocks.iter().any(|block| block.len() != 15)
            {
                return Err("motor feedback response count/length mismatch".into());
            }
            Some(blocks)
        } else {
            None
        };
        let requested = Instant::now();
        let blocks = controller.sync_read_raw_data(&[200], 56, 15)?;
        if blocks.len() != 1 {
            return Err("IMU response count mismatch".into());
        }
        let raw = &blocks[0];
        imu.observe(Frame::feetech(raw), requested);
        let sample = imu.sample(Instant::now());
        println!("{}", serde_json::json!({
            "elapsed_ms":start.elapsed().as_millis(),"raw":raw,
            "unix_ms":SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
            "index":index,"mixed_feedback":mixed,"motor_feedback":feedback,
            "ready":sample.ready,"gyro_rad_s":sample.data.gyro,
            "quat_wxyz":sample.data.quat,"gravity":sample.data.gravity,
            "transaction_us":requested.elapsed().as_micros()
        }));
        if let Some(remaining) = period.checked_sub(tick.elapsed()) {
            std::thread::sleep(remaining);
        }
    }
    // Confirm torque is still off after the run; never attempt to change it here.
    for id in duck_control::JOINT_IDS {
        let blocks = controller.sync_read_raw_data(&[id], 40, 1)?;
        if blocks.len() != 1 || blocks[0].as_slice() != [0] {
            return Err(format!("servo {id}: final torque not confirmed off: {blocks:?}").into());
        }
        println!("{}", serde_json::json!({"servo_id":id,"torque_enable":0,"phase":"final"}));
    }
    Ok(())
}
