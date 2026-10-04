#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderGroup {
    None,
    One(ColliderKind),
    Two(ColliderKind, ColliderKind),
    All,
}

impl std::fmt::Display for ColliderGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColliderGroup::None => write!(f, "None"),
            ColliderGroup::One(k) => write!(f, "One( {k} )"),
            ColliderGroup::Two(k1, k2) => write!(f, "Two( {k1}, {k2} )"),
            ColliderGroup::All => {
                write!(f, "All( ")?;
                if let Some((last, head)) = ColliderKind::ALL.split_last() {
                    for k in head {
                        write!(f, "{k}, ")?;
                    }
                    write!(f, "{last} ")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl ColliderGroup {
    pub fn contains(&self, kind: ColliderKind) -> bool {
        match self {
            ColliderGroup::None => false,
            ColliderGroup::One(k) => *k == kind,
            ColliderGroup::Two(a, b) => *a == kind || *b == kind,
            ColliderGroup::All => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderKind {
    Hit,
    Hurt,
    Push,
}

impl std::fmt::Display for ColliderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColliderKind::Hit => write!(f, "Hit"),
            ColliderKind::Hurt => write!(f, "Hurt"),
            ColliderKind::Push => write!(f, "Push"),
        }
    }
}

impl ColliderKind {
    pub const ALL: [ColliderKind; 3] = [ColliderKind::Hit, ColliderKind::Hurt, ColliderKind::Push];
}
