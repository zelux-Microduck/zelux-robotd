//! Read-only IMU transaction plans, using the existing motor controller's reader.
//! Framing, response IDs and CRC remain the bus library's responsibility.
use super::{
    DEVICE_ID, DYNAMIXEL_ADDRESS, DYNAMIXEL_LENGTH, FEETECH_ADDRESS, FEETECH_LENGTH, Frame,
    STATUS_ADDRESS,
};
use crate::io::{IoError, Result};

fn one(blocks: Vec<Vec<u8>>, length: u8) -> Result<Vec<u8>> {
    if blocks.len() != 1 {
        return Err(IoError::ShortRead {
            what: "IMU-DXL response count",
            expected: 1,
            got: blocks.len(),
        });
    }
    let block = blocks.into_iter().next().expect("count checked");
    if block.len() != usize::from(length) {
        return Err(IoError::ShortRead {
            what: "IMU-DXL response length",
            expected: usize::from(length),
            got: block.len(),
        });
    }
    Ok(block)
}

pub(crate) fn feetech(
    mut read: impl FnMut(&[u8], u8, u8) -> Result<Vec<Vec<u8>>>,
) -> Result<Frame> {
    let block = one(
        read(&[DEVICE_ID], FEETECH_ADDRESS, FEETECH_LENGTH)?,
        FEETECH_LENGTH,
    )?;
    Frame::feetech(&block)
}

pub(crate) fn dynamixel(
    mut read: impl FnMut(&[u8], u8, u8) -> Result<Vec<Vec<u8>>>,
) -> Result<Frame> {
    let before = one(read(&[DEVICE_ID], STATUS_ADDRESS, 1)?, 1)?;
    let core = one(
        read(&[DEVICE_ID], DYNAMIXEL_ADDRESS, DYNAMIXEL_LENGTH)?,
        DYNAMIXEL_LENGTH,
    )?;
    let after = one(read(&[DEVICE_ID], STATUS_ADDRESS, 1)?, 1)?;
    // A reset/not-ready transition on either side invalidates the measurement.
    Frame::dynamixel(&core, &[before[0] | after[0]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imu_dxl::ImuDxl;
    use std::collections::VecDeque;
    use std::time::{Duration, Instant};

    fn sample_ready(frame: Result<Frame>) -> bool {
        let mut imu = ImuDxl::new([1.0, 0.0, 0.0, 0.0], Duration::from_millis(100)).unwrap();
        let now = Instant::now();
        imu.observe(frame, now);
        imu.sample(now).ready
    }

    #[test]
    fn feetech_requests_only_imu_id_and_complete_block_once() {
        let mut calls = Vec::new();
        let frame = feetech(|ids, address, length| {
            calls.push((ids.to_vec(), address, length));
            Ok(vec![vec![0; 15]])
        });
        assert!(sample_ready(frame));
        assert_eq!(calls, vec![(vec![200], 56, 15)]);
    }

    #[test]
    fn feetech_rejects_missing_extra_short_and_oversized_responses() {
        for blocks in [
            vec![],
            vec![vec![0; 15]; 2],
            vec![vec![0; 14]],
            vec![vec![0; 16]],
        ] {
            assert!(feetech(|_, _, _| Ok(blocks.clone())).is_err());
        }
        assert!(feetech(|_, _, _| Err(IoError::Simulated)).is_err());
    }

    #[test]
    fn feetech_fault_invalidates_ready_and_duplicate_cannot_recover_it() {
        let now = Instant::now();
        let mut imu = ImuDxl::new([1.0, 0.0, 0.0, 0.0], Duration::from_millis(100)).unwrap();
        imu.observe(feetech(|_, _, _| Ok(vec![vec![0; 15]])), now);
        assert!(imu.sample(now).ready);
        imu.observe(
            feetech(|_, _, _| Err(IoError::Simulated)),
            now + Duration::from_millis(10),
        );
        assert!(!imu.sample(now + Duration::from_millis(10)).ready);
        imu.observe(
            feetech(|_, _, _| Ok(vec![vec![0; 15]])),
            now + Duration::from_millis(20),
        );
        assert!(!imu.sample(now + Duration::from_millis(20)).ready);
        let mut next = vec![0; 15];
        next[12] = 1;
        imu.observe(
            feetech(|_, _, _| Ok(vec![next.clone()])),
            now + Duration::from_millis(30),
        );
        assert!(imu.sample(now + Duration::from_millis(30)).ready);
    }

    #[test]
    fn dynamixel_reads_status_core_status_on_same_reader() {
        let mut calls = Vec::new();
        let frame = dynamixel(|ids, address, length| {
            calls.push((ids.to_vec(), address, length));
            Ok(vec![vec![0; usize::from(length)]])
        });
        assert!(sample_ready(frame));
        assert_eq!(
            calls,
            vec![
                (vec![200], 143, 1),
                (vec![200], 124, 12),
                (vec![200], 143, 1)
            ]
        );
    }

    #[test]
    fn dynamixel_not_ready_on_either_side_rejects_sample() {
        for (before, after) in [(1, 0), (0, 1), (1, 1)] {
            let mut replies =
                VecDeque::from([vec![vec![before]], vec![vec![0; 12]], vec![vec![after]]]);
            assert!(!sample_ready(dynamixel(|_, _, _| Ok(replies
                .pop_front()
                .unwrap()))));
            assert!(replies.is_empty());
        }
    }

    #[test]
    fn dynamixel_stops_on_each_transaction_failure() {
        for failed in 0..3 {
            let mut calls = 0;
            let frame = dynamixel(|_, _, length| {
                let index = calls;
                calls += 1;
                if index == failed {
                    Err(IoError::Simulated)
                } else {
                    Ok(vec![vec![0; usize::from(length)]])
                }
            });
            assert!(frame.is_err());
            assert_eq!(calls, failed + 1);
        }
    }

    #[test]
    fn dynamixel_validates_each_response_before_next_transaction() {
        for failed in 0..3 {
            for count in [0, 2] {
                let mut calls = 0;
                let frame = dynamixel(|_, _, length| {
                    let index = calls;
                    calls += 1;
                    Ok(vec![
                        vec![0; usize::from(length)];
                        if index == failed { count } else { 1 }
                    ])
                });
                assert!(frame.is_err());
                assert_eq!(calls, failed + 1);
            }
            for delta in [-1, 1] {
                let mut calls = 0;
                let frame = dynamixel(|_, _, length| {
                    let index = calls;
                    calls += 1;
                    let length = if index == failed {
                        (i32::from(length) + delta) as usize
                    } else {
                        usize::from(length)
                    };
                    Ok(vec![vec![0; length]])
                });
                assert!(frame.is_err());
                assert_eq!(calls, failed + 1);
            }
        }
    }

    #[test]
    fn serial_transaction_time_is_part_of_sample_age() {
        let started = Instant::now();
        let mut imu = ImuDxl::new([1.0, 0.0, 0.0, 0.0], Duration::from_millis(100)).unwrap();
        imu.observe(feetech(|_, _, _| Ok(vec![vec![0; 15]])), started);
        assert!(!imu.sample(started + Duration::from_millis(100)).ready);
    }
}
