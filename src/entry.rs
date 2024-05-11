use crate::{unwrap, Key, Value};
use std::{
    any::{type_name, TypeId},
    collections::btree_map,
    fmt::{self, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A view into a single type in the `TypeSet`, which may be either vacant or occupied.
///
/// This type is constructed by [`TypeSet::entry`][crate::TypeSet::entry]
///
/// ## Examples
///
/// This is a somewhat contrived example that demonstrates matching on the [`Entry`]. Often,
/// [`Entry::or_insert`], [`Entry::or_insert_with`], and [`Entry::and_modify`] can achieve
/// comparable results. See those functions for further usage examples.
///
/// ```rust
/// use type_set::{TypeSet, entry::Entry};
/// let mut set = TypeSet::new().with("hello");
/// let (previous, current) = match set.entry::<&'static str>() {
///     Entry::Vacant(vacant_entry) => {
///         let current = vacant_entry.insert("entry was vacant");
///         (None, current)
///     }
///
///     Entry::Occupied(mut occupied_entry) => {
///         let previous = occupied_entry.insert("entry was occupied");
///         (Some(previous), occupied_entry.into_mut())
///     }
/// };
/// assert_eq!(previous, Some("hello"));
/// assert_eq!(*current, "entry was occupied");
/// ```
#[derive(Debug)]
pub enum Entry<'a, T> {
    /// A view into the location a T would be stored in the `TypeSet`. See [`VacantEntry`]
    Vacant(VacantEntry<'a, T>),

    /// A view into the location a T is currently stored in the `TypeSet`. See [`OccupiedEntry`]
    Occupied(OccupiedEntry<'a, T>),
}

/// A view into a vacant entry in a `TypeSet`.
///
/// It is part of the [`Entry`] enum.
pub struct VacantEntry<'a, T>(
    pub(super) btree_map::VacantEntry<'a, Key, Value>,
    PhantomData<T>,
);

impl<'a, T> fmt::Debug for VacantEntry<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry")
            .field(&type_name::<T>())
            .finish()
    }
}
/// A view into the location a T is stored
pub struct OccupiedEntry<'a, T>(
    pub(super) btree_map::OccupiedEntry<'a, Key, Value>,
    PhantomData<T>,
);

impl<'a, T: fmt::Debug> fmt::Debug for OccupiedEntry<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OccupiedEntry").field(self.0.get()).finish()
    }
}

impl<'a, T: Send + Sync + 'static> Entry<'a, T> {
    /// Ensures a value is in the `Entry` by inserting the provided `default` value if the Entry was
    /// previously vacant. Returns a mutable reference to the value.
    ///
    /// Prefer [`Entry::or_insert_with`] if constructing a T is expensive.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = type_set::TypeSet::new();
    /// assert_eq!(*set.entry().or_insert("hello"), "hello");
    /// assert_eq!(set.get::<&'static str>(), Some(&"hello"));
    /// assert_eq!(*set.entry().or_insert("world"), "hello");
    /// assert_eq!(set.get::<&'static str>(), Some(&"hello"));
    /// ```
    pub fn or_insert(self, default: T) -> &'a mut T {
        match self {
            Entry::Vacant(vacant) => vacant.insert(default),
            Entry::Occupied(occupied) => occupied.into_mut(),
        }
    }

    /// Ensures a value is in the `Entry` by inserting the provided value returned by the `default`
    /// function if the `Entry` was previously vacant. Returns a mutable reference to the value.
    ///
    /// Prefer this to [`Entry::or_insert`] if constructing a T is expensive.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = type_set::TypeSet::new();
    /// assert_eq!(*set.entry().or_insert_with(|| String::from("hello")), "hello");
    /// assert_eq!(set.get::<String>(), Some(&String::from("hello")));
    /// assert_eq!(*set.entry::<String>().or_insert_with(|| panic!("never called")), "hello");
    /// assert_eq!(set.get::<String>(), Some(&String::from("hello")));
    /// ```
    pub fn or_insert_with(self, default: impl FnOnce() -> T) -> &'a mut T {
        match self {
            Entry::Vacant(vacant) => vacant.insert(default()),
            Entry::Occupied(occupied) => occupied.into_mut(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the
    /// set using [`Entry::or_insert`] or [`Entry::or_insert_with`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = type_set::TypeSet::new().with(String::from("hello"));
    /// let value = set.entry::<String>()
    ///     .and_modify(|s| s.push_str(" world"))
    ///     .or_insert_with(|| String::from("greetings"));
    /// assert_eq!(value, "hello world");
    ///
    /// set.remove::<String>();
    /// let value = set.entry::<String>()
    ///     .and_modify(|s| s.push_str(" world"))
    ///     .or_insert_with(|| String::from("greetings"));
    /// assert_eq!(value, "greetings");
    /// ```
    #[must_use]
    pub fn and_modify(self, f: impl FnOnce(&mut T)) -> Self {
        match self {
            Entry::Vacant(vacant) => Entry::Vacant(vacant),
            Entry::Occupied(mut occupied) => {
                f(occupied.get_mut());
                Entry::Occupied(occupied)
            }
        }
    }

    pub(super) fn new(entry: btree_map::Entry<'a, TypeId, Value>) -> Self {
        match entry {
            btree_map::Entry::Vacant(vacant) => Self::Vacant(VacantEntry(vacant, PhantomData)),
            btree_map::Entry::Occupied(occupied) => {
                Self::Occupied(OccupiedEntry(occupied, PhantomData))
            }
        }
    }
}

impl<'a, T: Default + Send + Sync + 'static> Entry<'a, T> {
    /// Ensures a value is in the Entry by inserting the default value if vacant, and returns a
    /// mutable reference to the value.
    ///
    /// Equivalent to `.or_insert_with(Default::default)`
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = type_set::TypeSet::new();
    /// assert_eq!(*set.entry::<&'static str>().or_default(), "");
    /// set.insert("hello");
    /// assert_eq!(*set.entry::<&'static str>().or_default(), "hello");
    /// ```
    pub fn or_default(self) -> &'a mut T {
        #[allow(clippy::unwrap_or_default)]
        // this is the implementation of or_default so it can't call or_default
        self.or_insert_with(T::default)
    }
}

impl<'a, T: Send + Sync + 'static> VacantEntry<'a, T> {
    /// Sets the value of this entry to the provided `value`
    pub fn insert(self, value: T) -> &'a mut T {
        unwrap!(self.0.insert(Box::new(value)).downcast_mut())
    }
}

impl<'a, T: Send + Sync + 'static> OccupiedEntry<'a, T> {
    /// Gets a reference to the value in this entry
    #[must_use]
    pub fn get(&self) -> &T {
        unwrap!(self.0.get().downcast_ref())
    }

    /// Gets a mutable reference to the value in the entry
    ///
    /// If you need a reference to the `OccupiedEntry` that may outlive the
    /// destruction of the `Entry` value, see [`OccupiedEntry::into_mut`].
    #[must_use]
    pub fn get_mut(&mut self) -> &mut T {
        unwrap!(self.0.get_mut().downcast_mut())
    }

    /// Sets the value of the entry to `value`, returning the entry's previous value.
    pub fn insert(&mut self, value: T) -> T {
        *unwrap!(self.0.insert(Box::new(value)).downcast())
    }

    /// Take ownership of the value from this Entry
    #[allow(clippy::must_use_candidate)] // sometimes we just want to take the value out and drop it
    pub fn remove(self) -> T {
        *unwrap!(self.0.remove().downcast())
    }

    /// Converts the entry into a mutable reference to its value.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`OccupiedEntry::get_mut`].
    #[must_use]
    pub fn into_mut(self) -> &'a mut T {
        unwrap!(self.0.into_mut().downcast_mut())
    }
}

impl<'a, T: Send + Sync + 'static> Deref for OccupiedEntry<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, T: Send + Sync + 'static> DerefMut for OccupiedEntry<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
