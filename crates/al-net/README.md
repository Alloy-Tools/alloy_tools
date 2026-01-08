# 1. Overview
An end-to-end encryption system for easy, secure networked connections between devices and users. Built with commonly used cryptographic primitives with zero-trust architecture.

- **Zero Trust Model:** Server never sees plaintext data or encryption keys.
- **End-to-End Encryption:** All data encrypted client-side before transmission.
- **Cryptographic Agility:** Modular design allowing algorithm updates.
- **Forward Secrecy:** Ephemeral keys protect past communications
- **Post-Compromise Security:** Through Noise protocol ratchet patterns

**Current Algorithms:**
- **Key Exchange:** **X25519**
- **Encryption:** **ChaCha20-Poly1305**
- **Signatures:** **Ed25519**
- **Hashing:** **BLAKE2s**

**Dependencies:**
- **OPAQUE:** The [opaque_ke](https://docs.rs/opaque-ke/latest/opaque_ke/) crate.
- **Noise:** The [snow](https://docs.rs/snow/latest/snow/) crate.

# 2. Security Philosophy & Threat Model
## 2.1 Defense-in-Depth
The system implements multiple layers of cryptographic protection, ensuring that compromise at any single layer does not compromise the entire communication stream. Each layer provides distinct security guarantees, creating a comprehensive security envelope for network connections.
## 2.2 Zero-Trust
The system assumes no inherent trust in the network. All connections must authenticate, all messages must be encrypted and authenticated, and all sessions must enable forward secrecy.

## 2.3 Attack Vectors

| Vector                                    | Protection                                         | Effectiveness                                         |
| ----------------------------------------- | -------------------------------------------------- | ----------------------------------------------------- |
| **Eavesdropping**                         | **ChaCha20-Poly1305** encryption with 256-bit keys | Ciphertext reveals nothing about plaintext            |
| **Message Replay**                        | 96-bit monotonic nonces                            | Duplicate nonces rejected                             |
| **Man-in-the-Middle**                     | Noise Protocol mutual authentication               | MITM cannot establish valid session                   |
| **Key Compromise**<br>**Forward Secrecy** | Ephemeral Diffie-Hellman + periodic rekeying       | Past sessions remain secure                           |
| **Post-Compromise Security**              | Noise protocol ratchet mechanisms                  | After compromise and rekeying, sessions are resecured |
| **Identity Spoofing**                     | Ed25519 signatures over handshakes                 | Requires private key possession                       |
| **Denial of Service**                     | *Partially mitigated* though rate limiting         |                                                       |

# 3. Architecture
## 3.1 Noise Protocol
### 3.1.1 Why Noise?
The Noise Protocol Framework was chosen as the foundation for the system due to its flexibility and proven security properties with formal analysis. Noise provides the following improvements:
- **Simplicity:** Clear cryptographic operations
- **Modularity:** Separate handshake patterns for different security requirements
- **Identity Hiding:** Optional protection of static public keys
- **Formal Verification:** Multiple independent security proofs conducted.
## 3.2 Handshake Patterns
### 3.2.1 Default Pattern
**Pattern:** `Noise_XX`<br>
**Use Case:** General-purpose connections where both parties need to authenticate.
- **Mutual Authentication:** Both parties prove identity
- **Identity Hiding:** Static keys protected until encrypted
- **Perfect Forward Secrecy:** Ephemeral-ephemeral DH
- **Key Compromise Impersonation (KCI):** Compromised keys can't impersonate others
### 3.2.2 Alternate Pattern
**Pattern:** `Noise_IK`<br>
**Use Case:** Performance-critical applications where initiator identity is exposed.
- **One-Round Trip:** Faster connection establishment
- **Initiator Identity Exposure:** Initiator's static key is sent in plaintext
- **Perfect Forward Secrecy:** Ephemeral-ephemeral DH
### 3.2.3 OPAQUE Pattern
**Pattern:** `Noise_XX` with `OPAQUE`<br>
**Use Case:** User authentication without certificate infrastructure.
- **Password Authentication**: No server-side password exposure
- **Augmented PAKE**: Pre-computation attack resistance
- **All Noise XX benefits**: Plus password-based authentication

# 4. Cryptographic Primitives
## 4.1 Key Exchange | X25519
**Algorithm:** Elliptic Curve Diffie-Hellman over Curve25519
- **Security**: 128-bit security level
- **Side-Channel Resistance**: Constant-time implementation inherent
- **Ubiquity**: Widely adopted, multiple independent implementations
- **Small Keys**: 32-byte public keys
## 4.2 Signatures | Ed25519
- **Deterministic:** No entropy required for signing
- **Small Signatures:** 64 bytes vs 71-72 for ECDSA
- **Security:** 128-bit level with protection against fault attacks
## 4.3 Encryption | ChaCha20-Poly1305
### 4.3.1 Why ChaCha20-Poly1305 over AES-GCM?

| Criteria                | ChaCha20-<br>Poly1305 | XChaCha20-<br>Poly1305 | AES-GCM   | Advantage                                                                          |
| ----------------------- | --------------------- | ---------------------- | --------- | ---------------------------------------------------------------------------------- |
| Nonce Size              | 96-bit                | 192-bit                | 96-bit    | Nonce prevents key exposure with duplicate data                                    |
| Nonce Use               | Monotonic             | Random                 | Monotonic | Monotonic nonces are easier to prevent replay attacks and implement replay windows |
| Software Performance    | Excellent             | Excellent              | Good      | **ChaCha20:** Often outperforms                                                    |
| Hardware Performance    | Good                  | Good                   | Excellent | **AES-GCM:** Generally faster                                                      |
| Side-Channel Resistance | Excellent             | Excellent              | Good      | **ChaCha20:** No lookup tables                                                     |
Poly1305 provides 128-bit authentication tags, ensuring:
- **Message integrity** (any modification detected)
- **Message authenticity** (only key holders can generate valid tags)
- **Associated Data authentication** (context binding)
## 4.4 Hash Function | BLAKE2s
- **Performance:** Faster than **SHA-256** on all platforms
- **Simplicity:** Single algorithm for all hashing needs
- **Length Extension:** Not vulnerable, unlike **SHA-256**

# 5. Session Management & Forward Secrecy
## 5.1 Key Hierarchy
### 5.1.1 Static Identity | Long-Term
 - **X25519** for encryption
 - **Ed25519** for signatures
### 5.1.2 Session Keys | Ephemeral
- **e** *initiator ephemeral* (destroyed after handshake)
- **re** *responder ephemeral* (destroyed after handshake)
- **HKDF** key derivation
	- **ck** *chaining key* (persists for rekeying)
	- **k** *transport key* (rekeyed periodically)
	- **h** *handshake hash* (Used for authentication)
### 5.1.3 Active Transport Keys
- **Send key** is rotated every 2<sup>31</sup> messages or 24 hours
- **Recv key** is rotated every 2<sup>31</sup> messages or 24 hours
## 5.2 Forward Secrecy Guarantees
**Immediate Forward Secrecy**
- Provided by ephemeral-ephemeral Diffie-Hellman (ee)
- Compromise of static keys during session doesn't expose session
- Achieved in all handshake patterns

**Periodic Forward Secrecy**
- Transport keys are rekeyed every 2<sup>31</sup> messages or 24 hours
- Limits amount of data encrypted with a single key
- Compromise of transport key exposes only messages in current period

**Post-Compromise Security**
- After rekeying, future messages secure even if current keys compromised
- Achieved by deriving new keys from chaining key, not previous transport keys
## 5.2 Rekeying Strategy
### 5.2.1 Trigger Conditions
- **Message-based:** After 2<sup>31</sup> (≈2.1 billion) messages in one direction
- **Time-based:** After 24 hours since creation
- **Explicit:** Application requests rekeying for security
### 5.2.2 Rekeying Process
``` Rust
let (ck, k) = HKDF(ck, k);
nonce_counter = 0;
```
- No network round-trip required
- Synchronized between parties via protocol messages
- Maintains chaining key continuity for future rekeying

# 6. Authentication Framework
## 6.1 Multi-Factor Authentication Support
**Factor 1: Required**
- Ed25519 signatures for static identity
- Provides proof of private key

**Factor 2: Optional**
- OPAQUE protocol for password authentication
- Server never sees plaintext password
- Resistant to pre-computation attacks

**Factor 3: Optional**
- Integration with TPM or remote server for key protection
- Private keys never leave secure element
## 6.2 Identity Verification
**Certificate Pinning**
- **Known Keys:** A protected list of known and trusted static keys or fingerprints

# 7. Protocol Security Properties
## 7.1 Provable Security Guarantees
- **Key Secrecy**
	- **X25519** is a secure Diffie-Hellman function and **ChaCha20-Poly1305** is a secure AEAD, the Noise protocol provides strong confidentiality for transport messages.
- **Authentication**
	- The secure signature **Ed25519** with the secure key exchange **X25519** provides mutual authentication in `Noise_XX` and server authentication in `Noise_IK`.
- **Forward Secrecy**
	- Ephemeral key exchange and regular rekeying provides forward secrecy.
## 7.2 Security Reduction
**If:**
1. **X25519** is a secure DH function
2. **ChaCha20** is a secure PRF
3. **Poly1205** is a secure MAC
4. **Ed25519** is a secure signature scheme
5. **HKDF** is a secure KDF
**Then:**
- The system provides authenticated encryption with forward secrecy

# 8. Threat-Specific Protections
## 8.1 Replay Attacks
- 96-bit nonces
- Session-specific context in all cryptographic operations
## 8.2 Denial of Service
- Initial computational cost on client before server state
- Connection rate limiting

# 9. Looking Into
- **Post-Compromise Security** improvements through double ratchet instead of noise ratchet.
	- Add a ratchet trait. First make the ratchet that uses the **Noise** protocol internally. Then could add a double ratchet implementation later.
- Implementing the **Noise** and **OPAQUE** protocols manually rather than using crates.
- Replay window, eg last N messages, keep track of N and a bitmask of last N nonces received.