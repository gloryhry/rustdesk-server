# users API compatibility

This local package preserves the unchanged hbb_common dependency name and API while forwarding every operation to crates.io `uzers` 0.12.2. It contains no code from the vulnerable users 0.11 implementation. Features cache, mock and logging forward to uzers.

The local package retains version 0.11.0 for Cargo compatibility. Cargo audit does not audit this local package against registry users advisories; the actual implementation is the locked registry uzers package and is audited normally. `tests/unix_groups.rs` compares the live group list with `id -G nobody`, reproducing the old spurious root membership before this replacement. Current hbb_common uses UID/home lookup and does not call the affected group function.

When hbb_common adopts uzers directly, remove this facade and patch.
