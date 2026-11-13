# Upstream provenance

This repository was split from the Microduck runtime so `robotd` can be built, released and
versioned independently from Studio and device management.

- Original project: `pollen-robotics/microduck`
- Hardware adaptation source: `fanhao375/microduck`, branch `feetech`
- Imported archive: `https://codeload.github.com/fanhao375/microduck/zip/refs/heads/feetech`
- Imported archive SHA-256: `d6882e5796feafcac2acc823d6d9cded0dc949aa906fc8e3564f8e1a554e1b42`
- Import date: 2026-11-13
- License: Apache-2.0 (the upstream `LICENSE` and notices are retained)

The first Zelux change introduces an `ImuSource` boundary. The original combined Dynamixel
transaction remains the default; independent I2C/SPI/IPC implementations can replace only the
orientation source without taking ownership of the servo bus.
