# Changelog

## 0.3.0

### New Features

* Add ARMv7a (32-bit) architecture support (https://github.com/arceos-org/axcpu/pull/16).
* Add generic timer abstraction for AArch64 and ARM architectures (https://github.com/arceos-org/axcpu/pull/23).

### Breaking Changes

* Upgrade [page_table_multiarch](https://crates.io/crates/page_table_multiarch) crate to v0.6, which adds ARM support. (https://github.com/arceos-org/axcpu/pull/22).
* Upgrade [percpu](https://crates.io/crates/percpu) crate to v0.3, see [percpu v0.3.0 changelog](https://github.com/arceos-org/percpu/blob/main/CHANGELOG.md#030).

### Bug Fixes

* Fix kernel stack pointer save on LoongArch64 (https://github.com/arceos-org/axcpu/pull/14).
* Set sstatus::FS before fp restore/clear in `switch_to` on riscv64 (https://github.com/arceos-org/axcpu/pull/30).

## 0.2.2

### Bug Fixes

* Fix compile error on riscv when enabling `uspace` feature (https://github.com/arceos-org/axcpu/pull/12).

## 0.2.1

### Bug Fixes

* Pad TrapFrame to multiple of 16 bytes for riscv64 (https://github.com/arceos-org/axcpu/pull/11).

## 0.2.0

### Breaking Changes

* Upgrade `memory_addr` to v0.4.

### New Features

* Add FP state switch for riscv64 (https://github.com/arceos-org/axcpu/pull/2).
* Add hypervisor support for aarch64 (https://github.com/arceos-org/axcpu/pull/10).

### Other Improvements

* Export `save`/`restore` in FP states for each architecture.
* Improve documentation.

## 0.1.1

### New Features

* Add `init::init_percpu` for x86_64.

### Other Improvements

* Improve documentation.

## 0.1.0

Initial release.
