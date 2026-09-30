mod sealed {
    pub trait Sealed {}
}
pub trait AsSecurityLevel: sealed::Sealed + 'static {
    fn as_security_level() -> SecurityLevel;
}
#[derive(Debug, Clone, Copy)]
pub struct Ephemeral;
impl sealed::Sealed for Ephemeral {}
impl AsSecurityLevel for Ephemeral {
    fn as_security_level() -> SecurityLevel {
        SecurityLevel::Ephemeral
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Persistent;
impl sealed::Sealed for Persistent {}
impl AsSecurityLevel for Persistent {
    fn as_security_level() -> SecurityLevel {
        SecurityLevel::Encrypted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    Ephemeral,
    Encrypted,
}

impl SecurityLevel {
    pub fn is_ephemeral(&self) -> bool {
        matches!(self, SecurityLevel::Ephemeral)
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, SecurityLevel::Encrypted)
    }

    pub fn is_persistable(&self) -> bool {
        self.is_encrypted()
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            SecurityLevel::Ephemeral => "ephemeral",
            SecurityLevel::Encrypted => "encrypted",
        }
    }
}
