# 1. Alloy
[![MIT License](https://img.shields.io/badge/License-MIT-blue?logo=readthedocs&logoColor=white)](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-MIT) [![Apache 2.0 License](https://img.shields.io/badge/License-Apache_2.0-blue?logo=readthedocs&logoColor=white)](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-APACHE_2.0) [![Rust CI](https://github.com/Alloy-Tools/alloy_tools/actions/workflows/rust_ci.yml/badge.svg)](https://github.com/Alloy-Tools/alloy_tools/actions/workflows/rust_ci.yml)

**Alloy** is a collection of modular, extendable, interoperable Rust libraries designed to work together seamlessly while remaining independently useful. Like an emergent system, the components combine to create something larger than the individual parts.
## 1.1 Why Alloy?
Modern Rust applications often need to assemble components from different ecosystems. Alloy provides cohesive foundation where:
- **Components are designed to work together** from the ground up
- **APIs are consistent** across different domains
- **Incremental adoption** — use one crate or the entire library
- **Performance is prioritized** without sacrificing ergonomics
## 1.2 Features
### 1.2.1 Core Principles
Alloys cohesive ecosystem of Rust libraries share some common design principles:
- **Modular**: Only use what you need — minimal dependencies
- **Interoperable**: Components work well together and with the broader Rust ecosystem
- **Performant**: Zero-cost abstractions where possible
- **Ergonomic**: Developer-friendly APIs with sensible defaults
- **Well-tested**: Comprehensive unit test coverage
- **Secure**: Built with security in mind
### 1.2.2 Shared Infrastructure
All Alloy crates benefit from:
- Common serialization formats
- Standardized async patterns
- Shared configuration patterns

With plans for:
- Unified error handling
- Consistent logging/tracing
# 2. Crates
## 2.1 Crate Descriptions

| Docs                                                                                                                                                 | Description                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| [![al-core docs](https://img.shields.io/badge/al--core-grey?logo=readme)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-core)       | Core utilities, traits, and foundational types |
| [![al-crypto docs](https://img.shields.io/badge/al--crypto-grey?logo=readme)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-crypto) | Cryptographic primitives and protocols         |
| [![al-vault docs](https://img.shields.io/badge/al--vault-grey?logo=readme)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-vault)    | Secure secret management and key storage       |
| [![al-net docs](https://img.shields.io/badge/al--net-grey?logo=readme)](https://github.com/Alloy-Tools/alloy_tools/tree/main/crates/al-net)          | Secure network connections                     |
## 2.2 Roadmap
See [roadmap](https://github.com/Alloy-Tools/alloy_tools/blob/main/ROADMAP.md) for planned crates and features.
# 3. Getting Started
## 3.1 Installation
### 3.1.1 From Crates.io
The code must be cloned from the repository for now, but publishing to `Crates.io` is planned.
### 3.1.2 From Repository
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
### 3.1.3 Feature Flags
Most crates support optional features
```toml
al-core = { path = "../alloy_tools/crates/al-core", features = ["serde", "json"] }
```
## 3.2 Quick Examples
### 3.2.1 Using al-core
```rust
```
### 3.2.2 Combining Multiple Crates
```rust
```
# 4. Project Structure
```
alloy_tools/
├── Cargo.toml         # Workspace configuration
├── README.md          # This file
├── ROADMAP.md         # Development roadmap
├── LICENSE-APACHE_2   # Apache 2.0 license notice
├── LICENSE-MIT        # Mit license notice
└── crates/            # All library crates
   ├── al-core/        # Core utilities
   ├── al-crypto/      # Cryptography
   ├── al-vault/       # Secret management
   └── ...
```
# 5. License
All code is dual-licensed, at your option, under either of:
- [Apache 2.0 License](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-APACHE_2.0)
- [MIT License](https://github.com/Alloy-Tools/alloy_tools/blob/main/LICENSE-MIT)
# 6. Acknowledgments
- [![tynm version 0.2](https://img.shields.io/badge/tynm-0.2-blue?logo=rust)](https://docs.rs/tynm/0.2/tynm/index.html) — Used to get simple types names that include
- [![serde version 1.0.219](https://img.shields.io/badge/serde-1.0.219-blue?logo=rust)](https://docs.rs/serde/1.0.219/serde/) — Used for serialization framework
- [![erased-serde version 0.4.6](https://img.shields.io/badge/erased--serde-0.4.6-blue?logo=rust)](https://docs.rs/erased-serde/0.4.6/erased_serde/) — Used for type erased serialization
- [![tokio version 1](https://img.shields.io/badge/tokio-1-blue?logo=rust)](https://docs.rs/tokio/latest/tokio/) — Used for threads and asynchronous runtime
- [![once_cell version 1.21.3](https://img.shields.io/badge/once__cell-1.21.3-blue?logo=rust)](https://docs.rs/crate/once_cell/1.21.3) — Used for lazy static globals