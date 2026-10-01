# atty API compatibility

This local package implements the small atty Stream/is/isnt API with stable `std::io::IsTerminal`. It contains no old atty code or unsafe pointer operations. Rust 1.70 or later is required.

The version 0.2.14 only preserves compatibility with unchanged clap/build/logger dependencies. Cargo audit does not audit this local package as registry atty; the implementation is the toolchain standard library. The old alignment issue concerns Windows with a custom allocator, and is not claimed as a reproduced Linux service exploit. Real Linux CLI pipe and pseudo-terminal behavior is verified by tests/terminal_cli.rs and tests/dependencies/terminal.py. No Windows server binary test is claimed.

Remove this facade when upstream consumers move to std::io::IsTerminal.
