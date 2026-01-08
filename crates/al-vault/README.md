# 1. Overview
Vaults provide protection for secrets using a layered security approach, with dedicated safeguards for data in memory and at rest.
## 1.1 Features
- **Cryptographic Primitives**
	- **Key Derivation:** Argon2ID through the [argon2](https://docs.rs/argon2/latest/argon2/) crate for the `PDK`.
	- **Encryption Algorithm:** All encryption operations use the **XChaCha20-Poly1305** authenticated encryption algorithm through the [chacha20poly1305](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/) crate.
	- **OPAQUE:** Key exchange protocol through the [opaque-ke](https://docs.rs/opaque-ke/latest/opaque_ke/) crate.
- **Memory Protection**
	- Ephemeral session keys are derived from an Argon2ID `PDK`.
	- [Secrets](https://docs.rs/secrets/latest/secrets/index.html) crate for `mlock`, `mprotect`, constant-time secret comparison, clone prevention, redacting from debug statements, and zeroing data on drop. Review the crate for a full list of features. 
	- `std::panic::catch_unwind()` used to safely call [secrets](https://docs.rs/secrets/latest/secrets/index.html) functions without panicking on failure.
- **At-Rest Encryption**
	- **Key Hierarchy:** Three-tier envelope encryption (`PDK`->`KEK`->`DEK`) where:
		- **Data Encryption Key (DEK):** Encrypts actual data, stored in OS keystores via the [keyring](https://crates.io/crates/keyring) crate.
		- **Key Encryption Key (KEK):** Encrypts `DEK`, stored encrypted according to chosen storage options.
		- **Password Derived Key (PDK):** Derived from user password, encrypts `KEK` for storage.
	- **KEK Storage Options:** The encrypted `KEK` storage is chosen from the following methods:
		- **TPM Binding** (if a Trusted Platform Module is available)
		- **Server Storage via OPAQUE** (if network access is available)
		- **OS Keystore** (as a fallback if neither a TPM nor network is available)
## 1.2 Algorithms
- **XChaCha20-Poly1305:** 256-bit key, 192-bit nonce
- **Argon2ID:** t=3, m=65536, p=1
- **Key derivation:** HKDF-BLAKE2s
- **MAC:** HMAC-BLAKE2s for config integrity
- **Key lengths:** 256-bit for all symmetric keys
## 1.3 Planned Features
- Rate limiting any password attempts
- Secure key caching within protected RAM
- Key versioning
- Automated key rotation
- Better key recovery protocols
- TOTP and other MFA methods
- Audit trails
- Encrypted vault transfers using `al-net`

# 2. In Use (RAM)
The [secrets](https://docs.rs/secrets/latest/secrets/index.html) crate is used to wrap data types with `SecureData<T: Zeroize>` which utilizes secrets internally to use `mlock` to prevent memory from being swapped out of RAM—preventing secrets from being leaked out of the process's virtual address space—and `mprotect` to protect the memory from being written to or read.

The inner data is zeroed when `SecureData<T: Zeroize>` is dropped, erasing the secure data to prevent leaks.

# 3. At Rest (Disk)
## 3.1 Encryption Key Architecture
Vaults use a three-tier envelope encryption system to protect data at rest:
1. **Data Encryption Key (DEK):** A randomly generated key that directly encrypts the actual data. The encrypted DEK is stored on the OS keystores using the [keyring](https://crates.io/crates/keyring) crate. 
2. **Key Encryption Key (KEK):** A randomly generated key used to encrypt the DEK. The encrypted KEK is stored according to the security method chosen from the section below.
3. **Password Derived Key (PDK):** Generated from the user's password using the **Argon2ID** algorithm via the [argon2](https://docs.rs/argon2/latest/argon2/) crate. The `PDK` encrypts the KEK for storage.
## 3.2 KEK Storage & Protection
Users can select from the following options to store and protect their encrypted KEK. The options are provided in the order of security recommendation:
1. **TPM Binding:** If a Trusted Platform Module (TPM) is available, the encrypted KEK is sealed to the specific TPM chip and platform state, binding it to the device.
2. **Server Storage:** If a network connection is available, the encrypted KEK is stored on a dedicated server using the **OPAQUE** protocol for authenticated, password-hardened retrieval.
3. **Local OS Keystore:** If no other option is available, the encrypted KEK can be stored in the OS keystore using the [keyring](https://crates.io/crates/keyring) crate.
## 3.3 Encryption Algorithm
All cryptographic operations—data encryption with the `DEK`, `DEK` encryption with the `KEK`, and `KEK` encryption with the `PDK`—use the **XChaCha20-Poly1305** authenticated encryption algorithm through the [chacha20poly1305](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/) crate.
## 3.4 Unlock Flow
To access encrypted data, the user supplies their password. The system then performs the following steps using protected memory (`SecureData` wrappers):
1. Derives the `PDK` from the password using **Argon2ID**.
2. Retrieves  the `KEK` from storage (TPM, Server, OS Keystore) and decrypts it with the `PDK`.
3. Retrieves and decrypts the `DEK` using the `KEK`.
4. Uses the `DEK` to decrypt data.
5. Zeros all sensitive material: the password, `PDK`, `KEK`, and `DEK`.
## 3.5 Key Recovery
A set of recovery keys that decrypt a copy of the `DEK` are provided during initial key creation and replaced during rekeying events involving the `DEK`. These should be stored in a separate, secure location—perhaps even on paper or a USB.

A recovery key can be used to trigger a rekeying process that recreates the entire key hierarchy and re-encrypts all data with the new `DEK`, making the old `DEK` irrelevant.

# 4. Security Considerations
## 4.1 Assumptions
- OS kernel is not compromised
- Hardware is not physically tampered with
- TPM (if used) is genuine and properly configured
- User's password has sufficient entropy
## 4.2 Attack Surfaces & Protections
- **Memory Attacks:**
	- Swap file exposure: Prevented by mlock()
	- Process inspection: Mitigated by mprotect()
	- Hardware attacks: Partially mitigated with ephemeral session keys 
- **Storage Attacks**
	- Stolen media: Protected by encryption
	- Filesystem tampering: Detected by AEAD tags
	- Replay attacks: Prevented by nonces
- **Network Attacks**
	- Man-in-the-Middle: Prevented by OPAQUE's authentication
	- Server compromise: Limited by OPAQUE's security properties
## 4.3 Non-Goals
- Protection against root/kernel compromise
- Protection against hardware attacks (cold boot, DMA)
- Protection against compromised TPM manufacturer
- Quantum resistance (post-quantum crypto not implemented)

# 5. Looking into
## 5.1 General
- **Lazy Secrets:** load only when needed, then drop immediately (zeroing the data)
- **Password quality estimation**: Before accepting passwords for PDK derivation.
- **Maximum password attempts** with exponential backoff
- **Signal handler safety**: Ensure secrets aren't leaked in core dumps or via signal handlers. The `secrets` crate helps, but check if additional platform specific `madvise(MADV_DONTDUMP)` is needed.
- **CPU cache side-channels**: While `secrets` handles timing attacks, consider whether cache-based attacks (like Spectre) need mitigation for high-value secrets.
## 5.2 Concurrency/Multi-threading
**Need to ensure:**
- Thread-safe secret handling
- Cross-process protection if needed
- Fork safety (secrets shouldn't survive fork() into child)
## 5.3 Configuration Management
- Compute a MAC of `(argon2id_params, salt)`, and any other tamper-proof fields, using the `PDK`
- Store `(argon2id_params, salt, config_mac)`, potentially with the `KEK`?
- On relaunch: derive `PDK` and compute MAC of stored config, `(argon2id_params, salt)`
- Verify the computed and stored mac are equal