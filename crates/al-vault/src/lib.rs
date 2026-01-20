mod audit;
mod container;
mod crypto;
mod secrets;

pub use container::{
    secure_container::{
        EncryptedExt, EphemeralExt, SecureAccess, SecureContainer, ToSecureContainer,
    },
    security_level::{AsSecurityLevel, Encrypted, Ephemeral, SecurityLevel},
};
pub use crypto::{keys::Key, nonce::Nonce};
pub use secrets::{
    dynamic_secret::DynamicSecret,
    fixed_secret::FixedSecret,
    secure_ref::{SecureRef, Secureable},
};

#[cfg(test)]
mod tests {

    #[test]
    fn todo() {
        todo!()
    }
}
