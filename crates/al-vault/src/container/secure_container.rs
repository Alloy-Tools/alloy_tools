pub trait SecureContainer {
    type SecretType: SecureContainer + SecureAccess + ToSecureContainer;
    type InnerType;
    type SecurityLevel: crate::AsSecurityLevel;

    fn tag(&self) -> &str;
    fn access(&self) -> &Self::SecretType;
    //TODO: should this be in to container to something?
    fn to_box(
        self,
    ) -> Box<
        dyn SecureContainer<
            SecretType = Self::SecretType,
            InnerType = Self::InnerType,
            SecurityLevel = Self::SecurityLevel,
        >,
    >
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    fn security_level(&self) -> crate::SecurityLevel {
        <Self::SecurityLevel as crate::AsSecurityLevel>::as_security_level()
    }

    fn audit_access(&self, _operation: &str) {
        todo!()
        //lazy static AuditLog: Vec<AuditEntry> using audit_log!() macro
    }
}

pub trait SecureAccess: SecureContainer {
    type ResultType<R>;

    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R>;
    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R>;
}

pub trait ToSecureContainer: SecureContainer {
    fn as_container(
        &self,
    ) -> &dyn SecureContainer<
        SecretType = Self::SecretType,
        InnerType = Self::InnerType,
        SecurityLevel = Self::SecurityLevel,
    >
    where
        Self: Sized,
    {
        self
    }
}

pub trait EncryptedExt {
    type EphemeralType;
    fn to_ephemeral(self) -> Self::EphemeralType;
}

pub trait EphemeralExt {
    /*type EncryptedType;
    fn to_encrypted(self) -> Self::EncryptedType;*/
}
