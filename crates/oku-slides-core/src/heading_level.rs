//! Heading level of a paragraph, from 1 (largest) to 6.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{HeadingLevelError, HeadingLevelSnafu};

/// Highest heading level, as in CommonMark ATX headings.
const MAX_LEVEL: u8 = 6;

/// Heading level of a paragraph: 1 to 6, written `#` to `######`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "u8", into = "u8")]
#[schemars(with = "u8")]
pub struct HeadingLevel(u8);

impl HeadingLevel {
    /// Level `n`, or `None` outside 1 to 6.
    #[must_use]
    pub fn new(n: u8) -> Option<Self> {
        (1..=MAX_LEVEL).contains(&n).then_some(Self(n))
    }

    /// The level as a number, 1 to 6.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for HeadingLevel {
    type Error = HeadingLevelError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        Self::new(n).ok_or_else(|| HeadingLevelSnafu { value: n }.build())
    }
}

impl From<HeadingLevel> for u8 {
    fn from(h: HeadingLevel) -> Self {
        h.0
    }
}

#[cfg(test)]
#[path = "heading_level_tests.rs"]
mod heading_level_tests;
