/// Keyboard modifiers as a bitmask
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    // Flag constants for each modifier
    pub const SHIFT: Self = Self(1 << 0);
    pub const CTRL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const SUPER: Self = Self(1 << 3);

    pub const ALL: Self = Self(0b1111);

    /// Returns an empty set of modifiers.
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns the set containing all defined modifiers.
    #[inline]
    pub const fn all() -> Self {
        Self::ALL
    }

    /// Returns the raw bits of the modifiers.
    #[inline]
    pub fn bits(self) -> u8 {
        self.0
    }

    /// Converts a raw `u8` into a `KeyModifiers`, returning `None` if any
    /// undefined bits are set.
    #[inline]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Creates a `KeyModifiers` from a raw u8, truncating any undefined bits.
    #[inline]
    pub fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL.0)
    }

    /// Returns `true` if no flags are set.
    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all flags are set.
    #[inline]
    pub const fn is_all(self) -> bool {
        self.0 == Self::ALL.0
    }

    /// Returns `true` if any of the flags in `other` are also set in `self`.
    #[inline]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns `true` if all flags in `other` are set in `self`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Inserts the given modifiers in place.
    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Removes the given modifiers in place.
    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Toggles the specified flags in place.
    #[inline]
    pub fn toggle(&mut self, other: Self) {
        self.0 ^= other.0;
    }

    /// Sets or clears the specified flags based on `value`.
    #[inline]
    pub fn set(&mut self, other: Self, value: bool) {
        if value {
            self.insert(other);
        } else {
            self.remove(other);
        }
    }

    /// Returns an iterator over the set flags.
    pub fn iter(self) -> impl Iterator<Item = Self> {
        [Self::SHIFT, Self::CTRL, Self::ALT, Self::SUPER]
            .into_iter()
            .filter(move |&flag| self.contains(flag))
    }
}

// Bitwise operators and assignments
impl std::ops::BitAnd for KeyModifiers {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for KeyModifiers {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::BitOr for KeyModifiers {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for KeyModifiers {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitXor for KeyModifiers {
    type Output = Self;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for KeyModifiers {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl std::ops::Not for KeyModifiers {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        // mask to keep only defined bits
        Self(!self.0 & Self::ALL.0)
    }
}

// Display and formatting
impl std::fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for flag in self.iter() {
            if !first {
                f.write_str(" | ")?;
            }
            let name = match flag {
                Self::SHIFT => "SHIFT",
                Self::CTRL => "CTRL",
                Self::ALT => "ALT",
                Self::SUPER => "SUPER",
                _ => unreachable!(), // only iterates over known flags
            };
            f.write_str(name)?;
            first = false;
        }
        if first {
            f.write_str("(empty)")?;
        }
        Ok(())
    }
}

impl std::fmt::Binary for KeyModifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

// Add modifiers from an iterator
impl Extend<KeyModifiers> for KeyModifiers {
    fn extend<T: IntoIterator<Item = KeyModifiers>>(&mut self, iter: T) {
        for flags in iter {
            *self = *self | flags;
        }
    }
}

// Create modifiers from an iterator
impl FromIterator<KeyModifiers> for KeyModifiers {
    fn from_iter<T: IntoIterator<Item = KeyModifiers>>(iter: T) -> Self {
        let mut result = Self::empty();
        result.extend(iter);
        result
    }
}

// To/From u8
impl From<KeyModifiers> for u8 {
    fn from(km: KeyModifiers) -> u8 {
        km.bits()
    }
}

impl From<u8> for KeyModifiers {
    fn from(bits: u8) -> Self {
        Self::from_bits_truncate(bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags() {
        assert!(KeyModifiers::default().is_none());

        let modifiers = KeyModifiers::empty();
        assert_eq!(modifiers.bits(), 0u8);
        assert!(modifiers.is_none());

        let modifiers = KeyModifiers::all();
        assert_eq!(modifiers.bits(), 0b1111u8);
        assert!(modifiers.is_all());

        let modifiers = KeyModifiers::SHIFT;
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(!modifiers.contains(KeyModifiers::CTRL));
    }

    #[test]
    fn intersects() {
        let modifiers = KeyModifiers::SHIFT | KeyModifiers::CTRL;
        assert!(modifiers.intersects(KeyModifiers::SHIFT));
        assert!(!modifiers.intersects(KeyModifiers::ALT));
    }

    #[test]
    fn bits() {
        assert_eq!(KeyModifiers::SHIFT.bits(), 1u8);
        assert_eq!((KeyModifiers::SHIFT | KeyModifiers::CTRL).bits(), 3u8);

        assert_eq!(KeyModifiers::from_bits(0b0001u8), Some(KeyModifiers::SHIFT));
        assert_eq!(KeyModifiers::from_bits(0b10000u8), None); // undefined bit
        assert_eq!(
            KeyModifiers::from_bits_truncate(0b0001u8),
            KeyModifiers::SHIFT
        );
        // only valid bits remain
        assert_eq!(KeyModifiers::from_bits_truncate(0b10011u8).bits(), 0b0011u8);
        
    }

    #[test]
    fn bitwise() {
        // and
        let left = KeyModifiers::SHIFT | KeyModifiers::CTRL;
        let right = KeyModifiers::CTRL | KeyModifiers::ALT;
        let result = left & right;
        assert_eq!(result, KeyModifiers::CTRL);

        // or
        let left = KeyModifiers::SHIFT;
        let right = KeyModifiers::CTRL;
        let result = left | right;
        assert!(result.contains(KeyModifiers::SHIFT));
        assert!(result.contains(KeyModifiers::CTRL));

        // xor
        let left = KeyModifiers::SHIFT | KeyModifiers::CTRL;
        let right = KeyModifiers::CTRL | KeyModifiers::ALT;
        let result = left ^ right;
        assert!(result.contains(KeyModifiers::SHIFT));
        assert!(result.contains(KeyModifiers::ALT));
        assert!(!result.contains(KeyModifiers::CTRL));

        // not
        let modifiers = KeyModifiers::SHIFT;
        let negated = !modifiers;
        assert_eq!(negated.bits(), 0b1110u8);
    }

    #[test]
    fn modifing() {
        let mut modifiers = KeyModifiers::SHIFT;
        assert!(!modifiers.contains(KeyModifiers::CTRL));
        modifiers.insert(KeyModifiers::CTRL);
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));

        modifiers.remove(KeyModifiers::SHIFT);
        assert!(!modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));

        modifiers.toggle(KeyModifiers::CTRL);
        assert!(!modifiers.contains(KeyModifiers::SHIFT));
        assert!(!modifiers.contains(KeyModifiers::CTRL));

        modifiers.toggle(KeyModifiers::SHIFT);
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(!modifiers.contains(KeyModifiers::CTRL));

        modifiers.set(KeyModifiers::CTRL, true);
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));

        modifiers.set(KeyModifiers::SHIFT, false);
        assert!(!modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));
    }

    #[test]
    fn from() {
        let modifiers = KeyModifiers::SHIFT | KeyModifiers::CTRL;
        assert_eq!(u8::from(modifiers), 0b0011u8);
        assert_eq!(KeyModifiers::from(0b0011u8).bits(), 0b0011u8);

        let flags = vec![KeyModifiers::SHIFT, KeyModifiers::CTRL];
        let modifiers = flags.into_iter().collect::<KeyModifiers>();
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));
    }

    #[test]
    fn iter() {
        let modifiers = KeyModifiers::empty();
        let vec: Vec<_> = modifiers.iter().collect();
        assert_eq!(vec.len(), 0);


        let modifiers = KeyModifiers::SHIFT | KeyModifiers::CTRL;
        let vec: Vec<_> = modifiers.iter().collect();
        assert_eq!(vec.len(), 2);
        assert!(vec.contains(&KeyModifiers::SHIFT));
        assert!(vec.contains(&KeyModifiers::CTRL));

        let mut modifiers = KeyModifiers::SHIFT;
        modifiers.extend(vec![KeyModifiers::CTRL, KeyModifiers::ALT]);
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));
        assert!(modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn display() {
        let mut modifiers = KeyModifiers::empty();
        assert_eq!(format!("{}", modifiers), "(empty)");

        modifiers.insert(KeyModifiers::SHIFT);
        assert_eq!(format!("{}", modifiers), "SHIFT");

        modifiers.insert(KeyModifiers::CTRL);
        let display = format!("{}", modifiers);
        assert!(display.contains("SHIFT"));
        assert!(display.contains("CTRL"));
        assert!(display.contains("|"));
    }

    #[test]
    fn equality() {
        assert_eq!(
            KeyModifiers::SHIFT | KeyModifiers::CTRL,
            KeyModifiers::SHIFT | KeyModifiers::CTRL
        );
        assert_ne!(KeyModifiers::SHIFT, KeyModifiers::CTRL);
    }
}
