mod container;
mod keys;
mod secrets;

pub use container::{
    secure_container::SecureContainer,
    security_level::{AsSecurityLevel, Encrypted, Ephemeral, SecurityLevel},
};
pub use keys::KEY_LENGTH;
pub use secrets::{
    dynamic_secret::DynamicSecret,
    fixed_secret::FixedSecret,
    secure_ref::{SecureRef, Secureable},
};

#[cfg(test)]
mod tests {

    #[test]
    fn todo() {}
}
