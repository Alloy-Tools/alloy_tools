mod audit;
mod container;
mod crypto;
mod async_executor;
mod secrets;


pub use audit::{AuditEntry, AuditError, AuditLog, AUDIT_LOG, AUDIT_LOG_CAPACITY};
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
