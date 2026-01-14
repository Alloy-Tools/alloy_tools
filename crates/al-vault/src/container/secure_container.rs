pub trait SecureContainer<L: crate::AsSecurityLevel> {
    type InnerType;
    type OutputType;
    type ResultType<R>;
    fn tag(&self) -> &str;
    fn audit_access(&self, operation: &str);
    //TODO: rename `new` & `take` to be more inline with actual function
    /// Will consume the data in `inner`, zeroing it before dropping
    fn new(inner: Self::InnerType, tag: impl Into<String>) -> Self::OutputType;
    /// Will zero out the data in `inner` after taking it
    fn take(inner: &mut Self::InnerType, tag: impl Into<String>) -> Self::OutputType;
    fn with<R>(&self, f: impl FnOnce(&Self::InnerType) -> R) -> Self::ResultType<R>;
    fn with_mut<R>(&mut self, f: impl FnOnce(&mut Self::InnerType) -> R) -> Self::ResultType<R>;

    fn security_level() -> crate::SecurityLevel {
        L::as_security_level()
    }
}
