/*
    1. Make a connection initiator that takes a handshake pattern (server can allow certain patterns)
    2. Will take an address, handle the handshake, and return the ephemeral session key and transport
    3. Be able to send `Data<S: ProtectedState>`, or recv it and proccess it.
    4. The data can now be any `Vec<u8>` (anything) encrypted. Send and handle `al-core` commands.
    5. Setup simple TCP VOIP with `al-core` events.
*/

// Following the Noise protocol specification: noiseprotocol.org/noise.html

const KEY_SIZE: usize = 32;
const DOUBLE_KEY_SIZE: usize = 2 * KEY_SIZE;
const TRIPLE_KEY_SIZE: usize = 3 * KEY_SIZE;
const DHLEN: usize = KEY_SIZE; // Must be 32 or greater
const HASHLEN: usize = KEY_SIZE; // Noise has HASHLEN 32 for BLAKE2s
const BLOCKLEN: usize = 64; // Noise has BLOCKLEN 64 for BLAKE2s

mod noise;

pub use noise::{
    cipher_state::{CipherState, CipherStateReturn},
    handshake_pattern::{HandshakePattern, HandshakeToken},
    handshake_state::HandshakeState,
    key_pair::{KeyPair, PublicKey},
    noise_error::NoiseError,
    symmetric_state::{SplitResult, SymmetricState},
};
