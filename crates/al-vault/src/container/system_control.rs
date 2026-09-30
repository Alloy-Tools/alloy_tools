use crate::{AsSecurityLevel, FixedSecret, SecureContainer};

//TODO: pub struct SystemKey<K: KeyType, const N: usize>(SystemSecret<FixedSecret<N, Ephemeral>>, PhantomData<K>);

//TODO system should be able to encrypt `Ephemeral` secrets by accessing them with `SystemPrivilege`

mod sealed {
    pub trait SystemPrivilege {}
    //pub trait InternalSystemPrivilege {}
}

pub trait SystemPrivilege: sealed::SystemPrivilege {
    fn access<const N: usize, L: AsSecurityLevel, R>(&self, secret: &SystemSecret<N, L>) -> R;
}
//trait InternalSystemPrivilege: sealed::InternalSystemPrivilege {}

pub enum SystemOperation {
    DecryptEnvelope(Vec<u8>),
}

impl sealed::SystemPrivilege for SystemOperation {}
/*impl SystemPrivilege for SystemOperation {
    fn access<T: SecureableBytes, L: AsSecurityLevel, R>(&self, secret: &SystemSecret<T, L>) -> R {
        match self {
            SystemOperation::DecryptEnvelope(cipher_text) => secret.0,
        }
    }
}*/

/// Wraps a `SecureBytes` to ensure only items with `SystemPrivilege` can access it
pub struct SystemSecret<const N: usize, L: AsSecurityLevel>(FixedSecret<N, L>);

impl<const N: usize, L: AsSecurityLevel> SystemSecret<N, L> {
    pub fn new(value: [u8; N], tag: impl Into<String>) -> Self {
        Self(FixedSecret::new(value, tag))
    }

    pub fn system_access<R, P: SystemPrivilege>(&self, p: P) -> R {
        //TODO: log system access
        p.access(&self)
    }
}
