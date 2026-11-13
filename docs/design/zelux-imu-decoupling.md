# IMU decoupling contract

## Ownership

`robotd` remains the only owner of the servo bus and the 50 Hz safety loop. An IMU that shares
that bus is read in the same transaction. An IMU on an independent I2C/SPI transport may have a
separate adapter, but its sample is consumed by the same control-loop boundary.

## Runtime boundary

The policy and safety code consume `ImuData` only. `ImuSource` supplies:

- angular velocity in rad/s, trunk frame;
- projected gravity in the trunk frame;
- trunk-to-world quaternion;
- convergence/ready state;
- cumulative and consecutive stale-sample counts.

`SplitImuIo<B, S>` composes the selected motor backend and IMU source. Motor commands always go
to `B`; only IMU fields are replaced by `S`. The default `EmbeddedImuSource` preserves the original
single-transaction Dynamixel behaviour.

## Compatibility rule

Changing an IMU driver must not change joint ordering, policy observations, units, coordinate
frames, or safety semantics. A source is eligible for motion only after `ready=true`. Stale or
failed sources must never silently return the identity orientation as a live measurement.

## Next adapters

1. shared-memory/Unix-socket input for a supervised sensor service;
2. direct I2C adapters for QMI8658, MPU6050 and ICM42688;
3. driver descriptor and version fields in the public health contract;
4. configuration selection wired to Studio through `zeluxd`.
