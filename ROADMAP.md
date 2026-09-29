# 1. Alloy Roadmap
This document outlines the current development focus and future plans for the `Alloy` ecosystem of Rust libraries.

## 1.1 Current Phase: Foundation & Security
The main objective is to make foundational crates—al-core and al-tui—that will aid in the interoperability and development of future crates and their functionality while ensuring the ecosystem can be trusted if secrets need to be used—al-vault and al-net.
- **Focus**: Stabilizing core APIs, comprehensive unit test coverage
- **Key Deliverables**:
	- Foundation: `al-core v1.0.0`
	- Crypto primitives: `al-crypto v1.0.0`
	- Secure local storage: `al-vault v1.0.0`
	- Secure network transmission: `al-net v1.0.0`
## 1.2 Crate Specific Plans

| Crate       | Status | Planned Features                          |
| ----------- | ------ | ----------------------------------------- |
| `al-core`   | v0.5.0 | Event registration macros                 |
| `al-crypto` | v0.0.1 | Cryptographic primitives                  |
| `al-vault`  | v0.0.1 | Secure RAM & persistent storage, CLI tool |
| `al-net`    | v0.0.1 | Secure network transfer, CLI tool         |
| `al-tui`    | v0.0.1 | Simple terminal user interface            |
# 2. Completed Items
- 