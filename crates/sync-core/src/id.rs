use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use uuid::Uuid;

/// A typed entity ID wrapping a UUID v7.
///
/// The phantom type parameter `E` prevents accidental mixing of IDs
/// from different entity types at compile time. For example, an
/// `Id<OrderTag>` cannot be used where an `Id<FillTag>` is expected,
/// even though both contain a UUID internally.
///
/// IDs use UUID v7 which embeds a millisecond timestamp, giving
/// natural chronological ordering without a separate sequence field.
///
/// # Construction
///
/// ```rust
/// use sync_core::Id;
/// use uuid::Uuid;
///
/// struct MyEntity;
///
/// // Generate a new time-ordered ID
/// let id = Id::<MyEntity>::new_v7();
///
/// // Wrap an existing UUID
/// let uuid = Uuid::now_v7();
/// let id2 = Id::<MyEntity>::from_uuid(uuid);
/// assert_eq!(id2.as_uuid(), &uuid);
/// ```
///
/// # Serialization
///
/// Serializes as a plain UUID string, making it wire-compatible with any
/// system that understands UUIDs:
///
/// ```rust
/// use sync_core::Id;
///
/// struct OrderTag;
///
/// let id = Id::<OrderTag>::new_v7();
/// let json = serde_json::to_string(&id).unwrap();
/// let roundtripped: Id<OrderTag> = serde_json::from_str(&json).unwrap();
/// assert_eq!(id, roundtripped);
/// ```
///
/// # Type safety
///
/// The phantom parameter ensures IDs from different entities cannot be mixed:
///
/// ```compile_fail
/// use sync_core::Id;
///
/// struct OrderTag;
/// struct FillTag;
///
/// let order_id: Id<OrderTag> = Id::new_v7();
/// let fill_id: Id<FillTag> = order_id; // ERROR: mismatched types
/// ```
pub struct Id<E> {
    inner: Uuid,
    _phantom: PhantomData<fn() -> E>,
}

// Manual impls to avoid bounds on E
impl<E> Clone for Id<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for Id<E> {}

impl<E> PartialEq for Id<E> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<E> Eq for Id<E> {}

impl<E> PartialOrd for Id<E> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> Ord for Id<E> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<E> Hash for Id<E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<E> Id<E> {
    /// Create an `Id` from an existing UUID.
    ///
    /// ```rust
    /// use sync_core::Id;
    /// use uuid::Uuid;
    ///
    /// struct MyEntity;
    /// let uuid = Uuid::nil();
    /// let id = Id::<MyEntity>::from_uuid(uuid);
    /// assert_eq!(id.to_string(), "00000000-0000-0000-0000-000000000000");
    /// ```
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            inner: uuid,
            _phantom: PhantomData,
        }
    }

    /// Generate a new UUID v7 ID.
    ///
    /// UUID v7 embeds a Unix millisecond timestamp, so IDs sort
    /// chronologically by creation time.
    ///
    /// ```rust
    /// use sync_core::Id;
    ///
    /// struct MyEntity;
    /// let a = Id::<MyEntity>::new_v7();
    /// let b = Id::<MyEntity>::new_v7();
    /// assert!(a <= b); // monotonically increasing
    /// ```
    pub fn new_v7() -> Self {
        Self::from_uuid(Uuid::now_v7())
    }

    /// Get a reference to the inner UUID.
    ///
    /// ```rust
    /// use sync_core::Id;
    /// use uuid::Uuid;
    ///
    /// struct MyEntity;
    /// let uuid = Uuid::nil();
    /// let id = Id::<MyEntity>::from_uuid(uuid);
    /// assert_eq!(id.as_uuid(), &Uuid::nil());
    /// ```
    pub fn as_uuid(&self) -> &Uuid {
        &self.inner
    }
}

impl<E> fmt::Debug for Id<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.inner)
    }
}

impl<E> fmt::Display for Id<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<E> Serialize for Id<E> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, E> Deserialize<'de> for Id<E> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let uuid = Uuid::deserialize(deserializer)?;
        Ok(Self::from_uuid(uuid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EntityA;
    struct EntityB;

    #[test]
    fn id_equality() {
        let uuid = Uuid::now_v7();
        let id1: Id<EntityA> = Id::from_uuid(uuid);
        let id2: Id<EntityA> = Id::from_uuid(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn id_serialization_roundtrip() {
        let id: Id<EntityA> = Id::new_v7();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: Id<EntityA> = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn id_display() {
        let uuid = Uuid::now_v7();
        let id: Id<EntityA> = Id::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }
}
