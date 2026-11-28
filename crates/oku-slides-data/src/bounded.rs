//! Deserializers that stop a list or a map as soon as its length passes a bound.
//!
//! Used as `#[serde(deserialize_with = "bounded::vec::<_, _, { limits::MAX_ROWS }>")]`: an
//! oversized input fails while it is read, before the rest of it is parsed or allocated.
//!
//! Except inside an internally tagged enum such as `TableOp` (`tag = "op"`): serde first
//! buffers the whole variant, so op input is capped only by the size of the input in bytes.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

/// A list of at most `MAX` items, as a `Vec`.
///
/// # Errors
///
/// The deserializer's error, or an invalid length past `MAX` items.
pub fn vec<'de, D, T, const MAX: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_seq(Seq::<Vec<T>, T, MAX>(PhantomData))
}

/// A list of at most `MAX` items, as an `imbl::Vector`.
///
/// # Errors
///
/// The deserializer's error, or an invalid length past `MAX` items.
pub fn vector<'de, D, T, const MAX: usize>(deserializer: D) -> Result<imbl::Vector<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Clone,
{
    deserializer.deserialize_seq(Seq::<imbl::Vector<T>, T, MAX>(PhantomData))
}

/// A map of at most `MAX` entries, as an `imbl::OrdMap` (a repeated key keeps its last value).
///
/// # Errors
///
/// The deserializer's error, or an invalid length past `MAX` entries.
pub fn map<'de, D, K, V, const MAX: usize>(deserializer: D) -> Result<imbl::OrdMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord + Clone,
    V: Deserialize<'de> + Clone,
{
    deserializer.deserialize_map(Map::<imbl::OrdMap<K, V>, K, V, MAX>(PhantomData))
}

/// A map of at most `MAX` entries, as a `BTreeMap` (a repeated key keeps its last value).
///
/// # Errors
///
/// The deserializer's error, or an invalid length past `MAX` entries.
pub fn btree<'de, D, K, V, const MAX: usize>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    deserializer.deserialize_map(Map::<BTreeMap<K, V>, K, V, MAX>(PhantomData))
}

/// A string of at most `MAX` UTF-8 bytes, refused before it is copied.
///
/// # Errors
///
/// The deserializer's error, or an invalid length past `MAX` bytes.
pub fn string<'de, D, const MAX: usize>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(Text::<MAX>)
}

/// Visits a string, failing past `MAX` bytes.
struct Text<const MAX: usize>;

impl<const MAX: usize> Visitor<'_> for Text<MAX> {
    type Value = String;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a string of at most {MAX} bytes")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<String, E> {
        if text.len() > MAX {
            return Err(E::invalid_length(text.len(), &self));
        }
        Ok(text.to_owned())
    }
}

/// Visits a list into `C`, failing past `MAX` items.
struct Seq<C, T, const MAX: usize>(PhantomData<(C, T)>);

impl<'de, C, T, const MAX: usize> Visitor<'de> for Seq<C, T, MAX>
where
    C: Default + Extend<T>,
    T: Deserialize<'de>,
{
    type Value = C;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at most {MAX} items")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<C, A::Error> {
        let mut out = C::default();
        for _ in 0..MAX {
            match seq.next_element()? {
                Some(item) => out.extend(Some(item)),
                None => return Ok(out),
            }
        }
        match seq.next_element::<IgnoredAny>()? {
            Some(_) => Err(de::Error::invalid_length(MAX.saturating_add(1), &self)),
            None => Ok(out),
        }
    }
}

/// Visits a map into `C`, failing past `MAX` entries.
struct Map<C, K, V, const MAX: usize>(PhantomData<(C, K, V)>);

impl<'de, C, K, V, const MAX: usize> Visitor<'de> for Map<C, K, V, MAX>
where
    C: Default + Extend<(K, V)>,
    K: Deserialize<'de>,
    V: Deserialize<'de>,
{
    type Value = C;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at most {MAX} entries")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<C, A::Error> {
        let mut out = C::default();
        for _ in 0..MAX {
            match access.next_entry()? {
                Some(entry) => out.extend(Some(entry)),
                None => return Ok(out),
            }
        }
        match access.next_key::<IgnoredAny>()? {
            Some(_) => Err(de::Error::invalid_length(MAX.saturating_add(1), &self)),
            None => Ok(out),
        }
    }
}

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod bounded_tests;
