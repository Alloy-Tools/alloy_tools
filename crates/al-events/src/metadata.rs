use al_structures::traits::{AsAny, AsBytes};
use std::{fmt::Debug, time::{Duration, SystemTime}};
use uuid::Uuid;

//REVIEW: Merge into `al-structures` Header trait to allow variable sized headers?
pub trait MetaData: AsAny + Debug {
    fn eq_dyn(&self, other: &dyn MetaData) -> bool;

    fn buf_len(&self) -> usize;

    fn encode(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()>;

    fn decode_slice(buffer: &[u8]) -> Result<(Self, usize), Box<dyn std::error::Error>>
    where
        Self: Sized;

    fn decode_reader(reader: &mut dyn std::io::Read) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}

impl PartialEq for dyn MetaData {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CommandMeta {
    id: Uuid,
    correlation_id: Uuid,
    timestamp: TimeStamp,
    expected_version: Option<u64>,
}

impl Default for CommandMeta {
    fn default() -> Self {
        Self::new(Uuid::nil(), None)
    }
}

impl CommandMeta {
    const LOW_LEN: usize = 41;
    const HIGH_LEN: usize = 49;

    pub fn new(correlation_id: Uuid, expected_version: Option<u64>) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id,
            timestamp: TimeStamp::now(),
            expected_version,
        }
    }

    pub fn from(
        id: Uuid,
        correlation_id: Uuid,
        timestamp: TimeStamp,
        expected_version: Option<u64>,
    ) -> Self {
        Self {
            id,
            correlation_id,
            timestamp,
            expected_version,
        }
    }

    pub fn uuid(&self) -> &Uuid {
        &self.id
    }

    pub fn set_uuid(&mut self, id: Uuid) {
        self.id = id;
    }

    pub fn correlation_id(&self) -> &Uuid {
        &self.correlation_id
    }

    pub fn set_correlation_id(&mut self, correlation_id: Uuid) {
        self.correlation_id = correlation_id;
    }

    pub fn timestamp(&self) -> &TimeStamp {
        &self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: TimeStamp) {
        self.timestamp = timestamp;
    }

    pub fn expected_version(&self) -> &Option<u64> {
        &self.expected_version
    }

    pub fn set_expected_version(&mut self, expected_version: Option<u64>) {
        self.expected_version = expected_version;
    }
}

impl MetaData for CommandMeta {
    fn eq_dyn(&self, other: &dyn MetaData) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| self == o)
    }

    fn buf_len(&self) -> usize {
        if self.expected_version.is_some() {
            Self::HIGH_LEN
        } else {
            Self::LOW_LEN
        }
    }

    fn encode(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let copy = |buf: &mut [u8], flag: u8| {
            buf[..16].copy_from_slice(self.id.as_bytes());
            buf[16..32].copy_from_slice(self.correlation_id.as_bytes());
            buf[32..40].copy_from_slice(&self.timestamp.as_micros().to_be_bytes());
            buf[40] = flag;
        };
        if let Some(v) = self.expected_version {
            let mut buf = [0u8; Self::HIGH_LEN];
            copy(&mut buf, 1);
            buf[41..49].copy_from_slice(&v.to_be_bytes());
            writer.write_all(&buf)
        } else {
            let mut buf = [0u8; Self::LOW_LEN];
            copy(&mut buf, 0);
            writer.write_all(&buf)
        }
    }

    fn decode_slice(buffer: &[u8]) -> Result<(Self, usize), Box<dyn std::error::Error>> {
        if buffer.len() < Self::LOW_LEN {
            return Err(format!(
                "Metadata requires at least {} bytes, got {}",
                Self::LOW_LEN,
                buffer.len()
            )
            .into());
        }
        let mut offset = Self::LOW_LEN;
        let id = Uuid::from_bytes(buffer[..16].try_into()?);
        let correlation_id = Uuid::from_bytes(buffer[16..32].try_into()?);
        let timestamp = TimeStamp::from_bytes(&buffer[32..40])?;
        let expected_version = if buffer[40] == 1 {
            if buffer.len() < Self::HIGH_LEN {
                return Err(format!(
                    "Metadata requires at least {} bytes, got {}",
                    Self::HIGH_LEN,
                    buffer.len()
                )
                .into());
            }
            offset = Self::HIGH_LEN;
            Some(u64::from_be_bytes(buffer[41..49].try_into()?))
        } else {
            None
        };
        Ok((
            Self::from(id, correlation_id, timestamp, expected_version),
            offset,
        ))
    }

    fn decode_reader(reader: &mut dyn std::io::Read) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = [0u8; Self::HIGH_LEN];
        reader.read_exact(&mut buffer[..Self::LOW_LEN])?;
        if buffer[40] == 1 {
            reader.read_exact(&mut buffer[Self::LOW_LEN..])?;
        }
        let (meta, _) = Self::decode_slice(&buffer)?;
        Ok(meta)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct EventMeta {
    id: Uuid,
    correlation_id: Uuid,
    timestamp: TimeStamp,
}

impl Default for EventMeta {
    fn default() -> Self {
        Self::new(Uuid::nil())
    }
}

impl EventMeta {
    const BUF_LEN: usize = 40;

    pub fn new(correlation_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id,
            timestamp: TimeStamp::now(),
        }
    }

    pub fn from(id: Uuid, correlation_id: Uuid, timestamp: TimeStamp) -> Self {
        Self {
            id,
            correlation_id,
            timestamp,
        }
    }

    pub fn uuid(&self) -> &Uuid {
        &self.id
    }

    pub fn set_uuid(&mut self, id: Uuid) {
        self.id = id;
    }

    pub fn correlation_id(&self) -> &Uuid {
        &self.correlation_id
    }

    pub fn set_correlation_id(&mut self, correlation_id: Uuid) {
        self.correlation_id = correlation_id;
    }

    pub fn timestamp(&self) -> &TimeStamp {
        &self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: TimeStamp) {
        self.timestamp = timestamp;
    }
}

impl MetaData for EventMeta {
    fn eq_dyn(&self, other: &dyn MetaData) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| self == o)
    }

    fn buf_len(&self) -> usize {
        Self::BUF_LEN
    }

    fn encode(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut buf = [0u8; Self::BUF_LEN];
        buf[..16].copy_from_slice(self.id.as_bytes());
        buf[16..32].copy_from_slice(self.correlation_id.as_bytes());
        buf[32..40].copy_from_slice(&self.timestamp.as_micros().to_be_bytes());
        writer.write_all(&buf)
    }

    fn decode_slice(buffer: &[u8]) -> Result<(Self, usize), Box<dyn std::error::Error>> {
        if buffer.len() < Self::BUF_LEN {
            return Err(format!(
                "Metadata requires at least {} bytes, got {}",
                Self::BUF_LEN,
                buffer.len()
            )
            .into());
        }
        let id = Uuid::from_bytes(buffer[..16].try_into()?);
        let correlation_id = Uuid::from_bytes(buffer[16..32].try_into()?);
        let timestamp = TimeStamp::from_bytes(&buffer[32..40])?;
        Ok((Self::from(id, correlation_id, timestamp), Self::BUF_LEN))
    }

    fn decode_reader(reader: &mut dyn std::io::Read) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = [0u8; Self::BUF_LEN];
        reader.read_exact(&mut buffer)?;
        let (meta, _) = Self::decode_slice(&buffer)?;
        Ok(meta)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct QueryMeta {
    id: Uuid,
    correlation_id: Uuid,
    timestamp: TimeStamp,
    timeout: Option<Duration>,
}

impl Default for QueryMeta {
    fn default() -> Self {
        Self::new(Uuid::nil(), None)
    }
}

impl QueryMeta {
    const LOW_LEN: usize = 41;
    const HIGH_LEN: usize = 49;

    pub fn new(correlation_id: Uuid, timeout: Option<Duration>) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id,
            timestamp: TimeStamp::now(),
            timeout,
        }
    }

    pub fn from(
        id: Uuid,
        correlation_id: Uuid,
        timestamp: TimeStamp,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            id,
            correlation_id,
            timestamp,
            timeout,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    pub fn correlation_id(&self) -> &Uuid {
        &self.correlation_id
    }

    pub fn set_correlation_id(&mut self, correlation_id: Uuid) {
        self.correlation_id = correlation_id;
    }

    pub fn timestamp(&self) -> &TimeStamp {
        &self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: TimeStamp) {
        self.timestamp = timestamp;
    }

    pub fn timeout(&self) -> &Option<Duration> {
        &self.timeout
    }

    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
}

impl MetaData for QueryMeta {
    fn eq_dyn(&self, other: &dyn MetaData) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| self == o)
    }

    fn buf_len(&self) -> usize {
        if self.timeout.is_some() {
            Self::HIGH_LEN
        } else {
            Self::LOW_LEN
        }
    }

    fn encode(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let copy = |buf: &mut [u8], flag: u8| {
            buf[..16].copy_from_slice(self.id.as_bytes());
            buf[16..32].copy_from_slice(self.correlation_id.as_bytes());
            buf[32..40].copy_from_slice(&self.timestamp.as_micros().to_be_bytes());
            buf[40] = flag;
        };
        if let Some(d) = self.timeout {
            let mut buf = [0u8; Self::HIGH_LEN];
            copy(&mut buf, 1);
            let bytes = d.as_millis().min(u64::MAX as u128) as u64;
            buf[41..49].copy_from_slice(&bytes.to_be_bytes());
            writer.write_all(&buf)
        } else {
            let mut buf = [0u8; Self::LOW_LEN];
            copy(&mut buf, 0);
            writer.write_all(&buf)
        }
    }

    fn decode_slice(buffer: &[u8]) -> Result<(Self, usize), Box<dyn std::error::Error>> {
        if buffer.len() < Self::LOW_LEN {
            return Err(format!(
                "Metadata requires at least {} bytes, got {}",
                Self::LOW_LEN,
                buffer.len()
            )
            .into());
        }
        let mut offset = Self::LOW_LEN;
        let id = Uuid::from_bytes(buffer[..16].try_into()?);
        let correlation_id = Uuid::from_bytes(buffer[16..32].try_into()?);
        let timestamp = TimeStamp::from_bytes(&buffer[32..40])?;
        let timeout = if buffer[40] == 1 {
            if buffer.len() < Self::HIGH_LEN {
                return Err(format!(
                    "Metadata requires at least {} bytes, got {}",
                    Self::HIGH_LEN,
                    buffer.len()
                )
                .into());
            }
            offset = Self::HIGH_LEN;
            Some(Duration::from_millis(u64::from_be_bytes(
                buffer[41..49].try_into()?,
            )))
        } else {
            None
        };
        Ok((Self::from(id, correlation_id, timestamp, timeout), offset))
    }

    fn decode_reader(reader: &mut dyn std::io::Read) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buffer = [0u8; Self::HIGH_LEN];
        reader.read_exact(&mut buffer[..Self::LOW_LEN])?;
        if buffer[40] == 1 {
            reader.read_exact(&mut buffer[Self::LOW_LEN..])?;
        }
        let (meta, _) = Self::decode_slice(&buffer)?;
        Ok(meta)
    }
}

// ----- Timestamp -----
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeStamp(u64);

impl TimeStamp {
    pub fn now() -> Self {
        Self::from_system_time(SystemTime::now())
    }

    pub fn from_system_time(system_time: SystemTime) -> Self {
        Self(
            system_time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0),
        )
    }

    pub fn to_system_time(&self) -> SystemTime {
        std::time::UNIX_EPOCH + Duration::from_micros(self.0)
    }

    pub fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    pub fn to_micros(self) -> u64 {
        self.0
    }

    pub fn as_micros(&self) -> &u64 {
        &self.0
    }

    pub fn is_stale(&self, timeout: Duration) -> bool {
        let curr = Self::now();
        if *self < curr {
            false
        } else {
            let age = curr.as_micros() - self.as_micros();
            age > timeout.as_micros() as u64
        }
    }
}

impl AsBytes for TimeStamp {
    const LEN: usize = 8;

    fn to_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.0.to_be_bytes())
    }

    fn from_bytes(buffer: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::from_micros(u64::from_be_bytes(buffer.try_into()?)))
    }
}
