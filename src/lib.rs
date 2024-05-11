#![deny(
    clippy::dbg_macro,
    missing_copy_implementations,
    rustdoc::missing_crate_level_docs,
    missing_debug_implementations,
    nonstandard_style,
    unused_qualifications
)]
#![warn(missing_docs, clippy::pedantic, clippy::perf, clippy::cargo)]
#![allow(clippy::missing_panics_doc, clippy::module_name_repetitions)]
/*!

[`TypeSet`] is a collection for heterogeneous types. Each type can only exist once in the set, and
can only be retrieved by naming the type.

Because types can only be retrieved by naming them, rust's module system allows module-private
storage in a shared `TypeSet`.

Currently, this crate imposes `Send + Sync` bounds on the stored types, but future versions may
offer variants without those bounds and/or with Clone bounds.

Implementation is based on
- <https://github.com/hyperium/http/blob/master/src/extensions.rs>
- <https://github.com/kardeiz/type-map/blob/master/src/lib.rs>
- <https://github.com/http-rs/http-types/blob/main/src/extensions.rs>
*/
use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    fmt::{self, Formatter},
};

/// Types for interacting with a mutable view into a `TypeSet` for a given type
pub mod entry;
use entry::Entry;

type Value = Box<dyn Any + Send + Sync>;
type Key = TypeId;

macro_rules! unwrap {
    ($x:expr) => {
        match $x {
            #[cfg(debug_assertions)]
            x => x.unwrap(),
            #[cfg(not(debug_assertions))]
            x => unsafe { x.unwrap_unchecked() },
        }
    };
}
use unwrap;

/// A collection for heterogenous types
///
/// Note that there is currently no way to iterate over the collection, as there may be types stored
/// that cannot be named by the calling code
#[derive(Default)]
pub struct TypeSet(BTreeMap<Key, Value>);

fn key<T: 'static>() -> Key {
    TypeId::of::<T>()
}

impl TypeSet {
    /// Create an empty `TypeSet`.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Returns true if the `TypeSet` contains zero types.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of distinct types in this `TypeSet`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Gets the corresponding type in the set for in-place manipulation.
    ///
    /// See [`Entry`] for usage.
    pub fn entry<T: Send + Sync + 'static>(&mut self) -> Entry<'_, T> {
        Entry::new(self.0.entry(key::<T>()))
    }

    /// Insert a value into this `TypeSet`.
    ///
    /// If a value of this type already exists, it will be replaced and returned.
    ///
    /// ## Example
    /// ```rust
    /// let mut set = typeset::TypeSet::new().with("hello");
    /// let previous = set.insert("world");
    /// assert_eq!(set.get::<&'static str>(), Some(&"world"));
    /// assert_eq!(previous, Some("hello"));
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        match self.entry() {
            Entry::Vacant(v) => {
                log::trace!("inserting {}", std::any::type_name::<T>());
                v.insert(value);
                None
            }

            Entry::Occupied(mut o) => {
                log::trace!("replacing {}", std::any::type_name::<T>());
                Some(o.insert(value))
            }
        }
    }

    /// Chainable constructor to add a type to this `TypeSet`
    ///
    /// ## Example
    /// ```rust
    /// let set = typeset::TypeSet::new().with("hello");
    /// assert_eq!(set.get::<&'static str>(), Some(&"hello"));
    /// ```
    #[must_use]
    pub fn with<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.insert(value);
        self
    }

    /// Check if this `TypeSet` contains a value for type T
    ///
    /// ## Example
    ///
    /// ```rust
    /// let set = typeset::TypeSet::new().with("hello");
    /// assert!(set.contains::<&'static str>());
    /// assert!(!set.contains::<String>());
    /// ```
    #[must_use]
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        log::trace!(
            "contains {}?: {}",
            std::any::type_name::<T>(),
            self.0.contains_key(&TypeId::of::<T>())
        );
        self.0.contains_key(&key::<T>())
    }

    /// Immutably borrow a value that has been inserted into this `TypeSet`.
    #[must_use]
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        log::trace!("getting {}", std::any::type_name::<T>(),);
        self.0
            .get(&key::<T>())
            .map(|boxed| unwrap!(boxed.downcast_ref()))
    }

    /// Attempt to mutably borrow to a value that has been inserted into this `TypeSet`.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = typeset::TypeSet::new().with(String::from("hello"));
    /// if let Some(string) = set.get_mut::<String>() {
    ///     string.push_str(" world");
    /// }
    /// assert_eq!(set.get::<String>().unwrap(), "hello world");
    /// ```
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.0
            .get_mut(&key::<T>())
            .map(|boxed| unwrap!(boxed.downcast_mut()))
    }

    /// Remove a value from this `TypeSet`.
    ///
    /// If a value of this type exists, it will be returned.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = typeset::TypeSet::new().with("hello");
    /// assert_eq!(set.remove::<&'static str>(), Some("hello"));
    /// assert_eq!(set.remove::<&'static str>(), None);
    /// ```
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.0
            .remove(&key::<T>())
            .map(|boxed| *unwrap!(boxed.downcast()))
    }

    /// Get a value from this `TypeSet` or populate it with the provided default.
    ///
    /// Identical to [`Entry::or_insert`]
    ///
    /// If building T is expensive, use [`TypeSet::get_or_insert_with`] or [`Entry::or_insert_with`]
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = typeset::TypeSet::new();
    /// assert_eq!(set.get_or_insert("hello"), &mut "hello");
    /// assert_eq!(set.get_or_insert("world"), &mut "hello");
    /// ```
    pub fn get_or_insert<T: Send + Sync + 'static>(&mut self, default: T) -> &mut T {
        self.entry().or_insert(default)
    }

    /// Get a value from this `TypeSet` or populate it with the provided default function.
    ///
    /// Identical to [`Entry::or_insert_with`]
    ///
    /// Prefer this to [`TypeSet::get_or_insert`] when building type T is expensive, since it will only be
    /// executed when T is absent.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = typeset::TypeSet::new();
    /// assert_eq!(set.get_or_insert_with(|| String::from("hello")), "hello");
    /// assert_eq!(set.get_or_insert_with::<String>(|| panic!("this is never called")), "hello");
    /// ```
    pub fn get_or_insert_with<T: Send + Sync + 'static>(
        &mut self,
        default: impl FnOnce() -> T,
    ) -> &mut T {
        self.entry().or_insert_with(default)
    }

    /// Ensure a value is present by filling with [`Default::default`]
    ///
    /// Identical to [`Entry::or_default`].
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set = typeset::TypeSet::new().with(10usize);
    /// let ten: usize = *set.get_or_insert_default();
    /// assert_eq!(ten, 10);
    /// ```
    pub fn get_or_insert_default<T: Default + Send + Sync + 'static>(&mut self) -> &mut T {
        self.entry().or_default()
    }

    /// Merge another `TypeSet` into this one, replacing any collisions
    ///
    ///
    /// ## Example
    ///
    /// ```rust
    /// let mut set_a = typeset::TypeSet::new().with(8u8).with("hello");
    /// let set_b = typeset::TypeSet::new().with(32u32).with("world");
    /// set_a.merge(set_b);
    /// assert_eq!(set_a.get::<u8>(), Some(&8));
    /// assert_eq!(set_a.get::<u32>(), Some(&32));
    /// assert_eq!(set_a.get::<&'static str>(), Some(&"world"));
    /// ```
    pub fn merge(&mut self, other: TypeSet) {
        self.0.extend(other.0);
    }
}

impl fmt::Debug for TypeSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeSet").finish()
    }
}
