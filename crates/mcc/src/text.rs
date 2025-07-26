use std::{borrow::Cow, sync::Arc};

/// A reference-counted string.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text(Arc<str>);

impl Text {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<Cow<'_, str>> for Text {
    fn from(s: Cow<'_, str>) -> Self {
        Self(s.as_ref().into())
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for Text {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &self.0
    }
}
