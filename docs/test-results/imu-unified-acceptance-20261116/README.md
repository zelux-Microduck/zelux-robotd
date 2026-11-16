# IMU unified acceptance — bench archive

Date: 2026-11-16  
Target: `microduck@192.168.2.117`  
Status: **physical read-only sample campaign archived; unified acceptance remains incomplete.**

## Contents

- `raw/`: 11 original JSONL captures and their paired `.err` streams (including intentionally empty stderr files), copied from the target without reformatting. Total directory size: 6,634,517 bytes.
- `artifacts/imu-readonly-bench`: ARM64 bounded read-only diagnostic used for static samples; SHA-256 `0f2ceeac6e56e13078bbc8939bb00e47c8b531eb02bb3f0c5b01c16c65c961a8`.
- `artifacts/imu-readonly-mixed-bench`: ARM64 mixed servo-feedback/IMU read-only diagnostic; SHA-256 `03007c8e5a88e98ab20139b2004a5f8812d9eba6f34b07b4adc639e232a7666c`.
- `SHA256SUMS.txt`: checksums for every archived raw log and binary.
- Detailed chronological observations and safety state: [`../../test-plan/imu-unified-acceptance-20261116-evidence.md`](../../test-plan/imu-unified-acceptance-20261116-evidence.md).
- Acceptance cases and completion criteria: [`../../test-plan/imu-unified-acceptance-20261116.md`](../../test-plan/imu-unified-acceptance-20261116.md).

## Results snapshot

- Static, held-tilt, return-to-upright, horizontal yaw, dynamic roll, and dynamic pitch captures were collected with the bounded read-only utility. Each pass checks all 15 servo torque-enable registers before reading IMU data; the final longer runs check them again after acquisition.
- Held left tilt and forward pitch changed the derived gravity/orientation; both return samples were close to their respective upright references.
- Horizontal left turn produced a dominant positive Z gyro response and counter-rotation a negative response. The operator confirmed an approximately 90° turn; the measured yaw change was of similar magnitude. Return yaw residual was approximately 15° and was not instrument-verified.
- Dynamic roll showed dominant positive X while leaning left and negative X while returning. Dynamic pitch showed dominant negative Y while leaning forward and positive Y while returning. These X/Y physical signs conflict with the currently assumed right-handed body axes and remain **unresolved**, not accepted as correct. No mounting/configuration change was made.
- Mixed read-only load: 1,500 cycles at a 20 ms target, each reading 15 servo feedback blocks followed by the IMU. All frames were ready, the median interval was 20 ms, and maximum quaternion difference from the first frame was about 0.54°. It did not reproduce the earlier robotd static-pose anomaly; this does not establish that the issue is fixed or identify its cause.
- `cargo test -p duck-control imu_dxl --lib` on the Linux build host: 23 passed, 0 failed.
- At archive time the test port was free, `robotd` remained inactive, and the production config hash was unchanged: `0b54e2f79a6e84b19f1298713dcd01685d1b607addb9a66b5454e260d98f2cf2`.

## Not covered / still open

X/Y mounting-sign resolution; recurrence and root cause of the prior anomaly under the actual robotd control loop; deployed Agent/Studio save-and-restart management closure; hardware fault/recovery and model-switch acceptance; raw serial framing/noise/CRC injection fixture tests. The mixed utility does not issue target writes, run the policy/control loop, or reproduce the full production service.

## Integrity notes

Three long-capture stderr files (`imu-readonly-yaw-dynamic.err`, `imu-readonly-roll-dynamic.err`, and `imu-readonly-pitch-dynamic.err`) contain only the deliberate `capture_started` marker; the other paired `.err` files are empty. They are preserved as captured. JSONL and executable hashes are recorded in `SHA256SUMS.txt`.
