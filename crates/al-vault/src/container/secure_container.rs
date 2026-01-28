pub trait SecureContainer {
    type InnerType;
    type SecurityLevel: crate::AsSecurityLevel;

    fn tag(&self) -> &str;
    fn access_count(&self) -> u64;
    fn len(&self) -> usize;

    fn security_level(&self) -> crate::SecurityLevel {
        <Self::SecurityLevel as crate::AsSecurityLevel>::as_security_level()
    }
}

pub trait SecureAccess: SecureContainer {
    type ResultType<R>;
    type CopyResultType;

    fn copy(&self) -> Self::CopyResultType;

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R>;
    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R>;

    #[cfg(not(test))]
    fn audit_access(&self, access_count: u64, operation: &str) -> Result<(), crate::AuditError> {
        crate::AUDIT_LOG.log_entry(crate::AuditEntry::new(access_count, operation, self.tag()))
    }

    #[cfg(test)]
    fn audit_access(&self, access_count: u64, operation: &str) -> Result<(), crate::AuditError> {
        println!(
            "{}",
            crate::AuditEntry::new(access_count, operation, self.tag())
        );
        Ok(())
    }
}

/* TODO: handle enryption for encryptable types or downgrading to ephemeral
pub trait EncryptedExt {
    type EphemeralType;
    fn to_ephemeral(self) -> Self::EphemeralType;
}*/

/* TODO: would the ephemeral types have any extra functions?
pub trait EphemeralExt {
    type EncryptedType;
    fn to_encrypted(self) -> Self::EncryptedType;
}*/
