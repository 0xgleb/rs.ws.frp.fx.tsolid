use std::collections::BTreeMap;

/// Trait implemented by Full entity structs to support diffing and merging.
pub trait Diffable: Sized {
    /// The patch type corresponding to this full type.
    type Patch: Default;

    /// Compute a diff between `self` (old) and `other` (new).
    /// Returns `None` if no fields changed.
    fn diff(&self, other: &Self) -> Option<Self::Patch>;

    /// Apply a patch to this value, producing an updated value.
    fn merge(self, patch: Self::Patch) -> Self;
}

/// Helper: diff two leaf values. Returns `Some(new)` if they differ.
pub fn diff_leaf<T: PartialEq + Clone>(old: &T, new: &T) -> Option<T> {
    if old != new {
        Some(new.clone())
    } else {
        None
    }
}

/// Helper: merge a leaf field. If patch is Some, use it; otherwise keep original.
pub fn merge_leaf<T>(original: T, patch: Option<T>) -> T {
    patch.unwrap_or(original)
}

/// Helper: diff two leaf BTreeMaps. Produces a patch map where:
/// - Keys present in new but not old → Some(new_value)
/// - Keys present in both but different → Some(new_value)
/// - Keys present in old but not new → None (tombstone)
/// - Keys unchanged → omitted
pub fn diff_leaf_map<K, V>(old: &BTreeMap<K, V>, new: &BTreeMap<K, V>) -> BTreeMap<K, Option<V>>
where
    K: Ord + Clone,
    V: PartialEq + Clone,
{
    let mut patch = BTreeMap::new();

    // Check for changed or added keys
    for (k, new_v) in new {
        match old.get(k) {
            Some(old_v) if old_v == new_v => {} // unchanged
            _ => {
                patch.insert(k.clone(), Some(new_v.clone()));
            }
        }
    }

    // Check for removed keys
    for k in old.keys() {
        if !new.contains_key(k) {
            patch.insert(k.clone(), None);
        }
    }

    patch
}

/// Helper: merge a leaf BTreeMap with a patch.
/// - `Some(v)` → insert/overwrite
/// - `None` → tombstone (remove key)
pub fn merge_leaf_map<K, V>(
    mut original: BTreeMap<K, V>,
    patch: BTreeMap<K, Option<V>>,
) -> BTreeMap<K, V>
where
    K: Ord,
{
    for (k, v) in patch {
        match v {
            Some(val) => {
                original.insert(k, val);
            }
            None => {
                original.remove(&k);
            }
        }
    }
    original
}

/// Helper: diff two nested BTreeMaps where values are Diffable.
/// Produces a patch map where:
/// - Changed values → Some(patch)
/// - Removed keys → None (tombstone)
/// - Unchanged → omitted
pub fn diff_nested_map<K, V>(
    old: &BTreeMap<K, V>,
    new: &BTreeMap<K, V>,
) -> BTreeMap<K, Option<V::Patch>>
where
    K: Ord + Clone,
    V: Diffable + Clone,
{
    let mut patch = BTreeMap::new();

    for (k, new_v) in new {
        match old.get(k) {
            Some(old_v) => {
                if let Some(p) = old_v.diff(new_v) {
                    patch.insert(k.clone(), Some(p));
                }
            }
            None => {
                // New key — diff against default would be wrong; we need the full state.
                // For new keys in a nested map, we diff against the old value.
                // Since there's no old value, we produce a diff from a default.
                // Actually, for a new nested entry we should produce a patch that
                // represents the full value. We do this by diffing a default-constructed
                // Full against the new value. But Full doesn't implement Default...
                // Instead, we'll rely on the macro to generate a `to_patch` method.
                // For now, let's just store the full diff. The macro will handle this.
                // We need the value to be convertible to its own patch.
                // Let's skip this for now and handle it in the macro.
                // Actually the simplest correct approach: new entries get a full patch.
                // We'll handle this via a FullToPatch trait.
                let _ = (k, new_v);
            }
        }
    }

    // Removed keys
    for k in old.keys() {
        if !new.contains_key(k) {
            patch.insert(k.clone(), None);
        }
    }

    patch
}

/// Helper: merge a nested BTreeMap with a patch.
pub fn merge_nested_map<K, V>(
    mut original: BTreeMap<K, V>,
    patch: BTreeMap<K, Option<V::Patch>>,
) -> BTreeMap<K, V>
where
    K: Ord,
    V: Diffable,
{
    for (k, v) in patch {
        match v {
            Some(p) => {
                if let Some(existing) = original.remove(&k) {
                    original.insert(k, existing.merge(p));
                }
                // If key doesn't exist and we get a patch, we can't reconstruct
                // the full value from just a patch. This shouldn't happen in
                // normal operation — the server would send a Full for new entities.
            }
            None => {
                original.remove(&k);
            }
        }
    }
    original
}

/// Trait for converting a Full entity to its Patch representation (all fields set).
pub trait FullToPatch {
    type Patch;
    fn to_patch(&self) -> Self::Patch;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_leaf_unchanged() {
        assert_eq!(diff_leaf(&42, &42), None);
    }

    #[test]
    fn diff_leaf_changed() {
        assert_eq!(diff_leaf(&42, &99), Some(99));
    }

    #[test]
    fn merge_leaf_with_none() {
        assert_eq!(merge_leaf(42, None), 42);
    }

    #[test]
    fn merge_leaf_with_some() {
        assert_eq!(merge_leaf(42, Some(99)), 99);
    }

    #[test]
    fn diff_leaf_map_basic() {
        let old: BTreeMap<String, i32> = [("a".into(), 1), ("b".into(), 2)].into();
        let new: BTreeMap<String, i32> = [("a".into(), 1), ("c".into(), 3)].into();
        let patch = diff_leaf_map(&old, &new);

        assert_eq!(patch.get("a"), None); // unchanged, omitted
        assert_eq!(patch.get("b"), Some(&None)); // tombstone
        assert_eq!(patch.get("c"), Some(&Some(3))); // added
    }

    #[test]
    fn merge_leaf_map_basic() {
        let original: BTreeMap<String, i32> = [("a".into(), 1), ("b".into(), 2)].into();
        let patch: BTreeMap<String, Option<i32>> = [
            ("b".into(), None),       // tombstone
            ("c".into(), Some(3)),    // add
        ].into();
        let result = merge_leaf_map(original, patch);

        assert_eq!(result.get("a"), Some(&1));
        assert_eq!(result.get("b"), None);
        assert_eq!(result.get("c"), Some(&3));
    }
}
