use serde::ser::{Serialize, SerializeSeq};

/// An adapter that lets you serialize a [`salsa`] type (e.g. from
/// [`#[salsa::tracked]`][salsa::tracked] or [`#[salsa::input]`][salsa::input])
/// using serde.
///
/// You'll most likely want to derive this using
/// [`#[derive(mcc_macros::SerializeWithDatabase)]`][mcc_macros::SerializeWithDatabase].
pub trait SerializeWithDatabase {
    fn serialize_with_db<'a>(&'a self, db: &'a dyn salsa::Database) -> impl Serialize + 'a;
}

// impl<T: Serialize + ?Sized> SerializeWithDatabase for T {
//     fn serialize_with_db<'a>(&'a self, _: &'a dyn salsa::Database) -> impl Serialize + 'a {
//         self
//     }
// }

impl<T: SerializeWithDatabase> SerializeWithDatabase for &'_ [T] {
    fn serialize_with_db<'a>(&'a self, db: &'a dyn salsa::Database) -> impl Serialize + 'a {
        SerializeSlice { db, inner: self }
    }
}

impl<T: SerializeWithDatabase> SerializeWithDatabase for Vec<T> {
    fn serialize_with_db<'a>(&'a self, db: &'a dyn salsa::Database) -> impl Serialize + 'a {
        SerializeSlice {
            db,
            inner: &self[..],
        }
    }
}

struct SerializeSlice<'a, T: SerializeWithDatabase> {
    db: &'a dyn salsa::Database,
    inner: &'a [T],
}

impl<'a, T: SerializeWithDatabase> Serialize for SerializeSlice<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.inner.len()))?;
        for item in self.inner {
            state.serialize_element(&item.serialize_with_db(self.db))?;
        }
        state.end()
    }
}

/// Create a helper which uses deref specialisation to allow implementing
/// `SerializeWithDatabase` using `Serialize` only.
#[allow(unused)]
pub(crate) fn helper<T>(value: &T) -> SerializeWithDatabaseHelper<'_, T> {
    SerializeWithDatabaseHelper(SerializeHelper(value))
}

pub(crate) struct SerializeHelper<'a, T>(&'a T);

impl<'b, T: Serialize> SerializeWithDatabase for SerializeHelper<'b, T> {
    fn serialize_with_db<'a>(&'a self, _: &'a dyn salsa::Database) -> impl Serialize + 'a {
        self.0
    }
}

pub(crate) struct SerializeWithDatabaseHelper<'a, T>(SerializeHelper<'a, T>);

impl<'a, T> std::ops::Deref for SerializeWithDatabaseHelper<'a, T> {
    type Target = SerializeHelper<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: SerializeWithDatabase> SerializeWithDatabase for SerializeWithDatabaseHelper<'a, T> {
    fn serialize_with_db<'b>(&'b self, db: &'b dyn salsa::Database) -> impl Serialize + 'b {
        self.0.0.serialize_with_db(db)
    }
}
