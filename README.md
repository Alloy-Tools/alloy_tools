# What is the `Alloy` Library?
**Alloy** is a collection of modular, extendable, interoperable Rust libraries designed to work together seamlessly while remaining independently useful. Like an emergent system, the components combine to create something larger than the individual parts.
## Overview
Alloy provides a cohesive ecosystem of Rust libraries that share common design principles:
- **Modular**: Only use what you need
- **Interoperable**: Components work well together
- **Performant**: Zero-cost abstractions where possible
- **Ergonomic**: Developer-friendly APIs
- **Well-tested**: Comprehensive test coverage
## Crates
| Docs                                                                                                                                                 | Description |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| [![al-core docs](https://img.shields.io/badge/al--core-grey?logo=github)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-core)       |             |
| [![al-crypto docs](https://img.shields.io/badge/al--crypto-grey?logo=github)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-crypto) |             |
| [![al-vault docs](https://img.shields.io/badge/al--vault-grey?logo=github)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-vault)    |             |
| [![al-net docs](https://img.shields.io/badge/al--net-grey?logo=github)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-net)          |             |
## Installation
```bash
# Clone the repository
git clone https://github.com/Alloy-Tools/alloy_tools.git
cd alloy_tools

# Build all crates
cargo build

# Run tests
cargo test
```

Add specific crates to your `Cargo.toml`:
```toml
[dependencies]
al-core = { path = "../alloy_tools/crates/al-core" }
al-vault = { path = "../alloy_tools/crates/al-vault" }
```
## Quick Example
```rust

```
## Workspace Structure
```
alloy_tools/
├── Cargo.toml         # Workspace configuration
├── README.md          # This file
├── LICENSE-APACHE_2   # Apache 2.0 license notice
├── LICENSE-MIT        # Mit license notice
└── crates/            # All library crates
   ├── al-core/
   ├── al-crypto/
   ├── al-vault/
   └── ...
```
## License
All code is dual-licensed, at your option, under either of:
- [Apache 2.0 License](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-APACHE_2.0)
- [MIT License](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-MIT)
## Acknowledgments
- [![tynm version 0.2](https://img.shields.io/badge/tynm-0.2-blue?logo=rust)](https://docs.rs/tynm/0.2/tynm/index.html) — Used to get simple types names that include
- [![serde version 1.0.219](https://img.shields.io/badge/serde-1.0.219-blue?logo=rust)](https://docs.rs/serde/1.0.219/serde/) — Used for serialization framework
- [![erased-serde version 0.4.6](https://img.shields.io/badge/erased--serde-0.4.6-blue?logo=rust)](https://docs.rs/erased-serde/0.4.6/erased_serde/) — Used for type erased serialization
- [![tokio version 1](https://img.shields.io/badge/tokio-1-blue?logo=rust)](https://docs.rs/tokio/latest/tokio/) — Used for threads and asynchronous runtime
- [![once_cell version 1.21.3](https://img.shields.io/badge/once__cell-1.21.3-blue?logo=rust)](https://docs.rs/crate/once_cell/1.21.3) — Used for lazy static globals