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

pub trait ToSecureContainer: SecureContainer {
    fn as_container(
        &self,
    ) -> &dyn SecureContainer<InnerType = Self::InnerType, SecurityLevel = Self::SecurityLevel>
    where
        Self: Sized,
    {
        self
    }

    fn to_box(
        self,
    ) -> Box<dyn SecureContainer<InnerType = Self::InnerType, SecurityLevel = Self::SecurityLevel>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}
impl<T: SecureContainer> ToSecureContainer for T {}

pub trait EncryptedExt {
    type EphemeralType;
    fn to_ephemeral(self) -> Self::EphemeralType;
}

pub trait EphemeralExt {
    /*type EncryptedType;
    fn to_encrypted(self) -> Self::EncryptedType;*/
}
