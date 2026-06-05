//! Utilities for working with borrowed data.

#![stable(feature = "rust1", since = "1.0.0")]
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::{Deref, DerefPure};

/// A trait for borrowing data.
///
/// In Rust, it is common to provide different representations of a type for
/// different use cases. For instance, storage location and management for a
/// value can be specifically chosen as appropriate for a particular use via
/// pointer types such as [`Box<T>`] or [`Rc<T>`]. Beyond these generic
/// wrappers that can be used with any type, some types provide optional
/// facets providing potentially costly functionality. An example for such a
/// type is [`String`] which adds the ability to extend a string to the basic
/// [`str`]. This requires keeping additional information unnecessary for a
/// simple, immutable string.
///
/// These types provide access to the underlying data through references
/// to the type of that data. They are said to be ‘borrowed as’ that type.
/// For instance, a [`Box<T>`] can be borrowed as `T` while a [`String`]
/// can be borrowed as `str`.
///
/// Types express that they can be borrowed as some type `T` by implementing
/// `Borrow<T>`, providing a reference to a `T` in the trait’s
/// [`borrow`] method. A type is free to borrow as several different types.
/// If it wishes to mutably borrow as the type, allowing the underlying data
/// to be modified, it can additionally implement [`BorrowMut<T>`].
///
/// Further, when providing implementations for additional traits, it needs
/// to be considered whether they should behave identically to those of the
/// underlying type as a consequence of acting as a representation of that
/// underlying type. Generic code typically uses `Borrow<T>` when it relies
/// on the identical behavior of these additional trait implementations.
/// These traits will likely appear as additional trait bounds.
///
/// In particular `Eq`, `Ord` and `Hash` must be equivalent for
/// borrowed and owned values: `x.borrow() == y.borrow()` should give the
/// same result as `x == y`.
///
/// If generic code merely needs to work for all types that can
/// provide a reference to related type `T`, it is often better to use
/// [`AsRef<T>`] as more types can safely implement it.
///
/// [`Box<T>`]: ../../std/boxed/struct.Box.html
/// [`Mutex<T>`]: ../../std/sync/struct.Mutex.html
/// [`Rc<T>`]: ../../std/rc/struct.Rc.html
/// [`String`]: ../../std/string/struct.String.html
/// [`borrow`]: Borrow::borrow
///
/// # Examples
///
/// As a data collection, [`HashMap<K, V>`] owns both keys and values. If
/// the key’s actual data is wrapped in a managing type of some kind, it
/// should, however, still be possible to search for a value using a
/// reference to the key’s data. For instance, if the key is a string, then
/// it is likely stored with the hash map as a [`String`], while it should
/// be possible to search using a [`&str`][`str`]. Thus, `insert` needs to
/// operate on a `String` while `get` needs to be able to use a `&str`.
///
/// Slightly simplified, the relevant parts of `HashMap<K, V>` look like
/// this:
///
/// ```
/// use std::borrow::Borrow;
/// use std::hash::Hash;
///
/// pub struct HashMap<K, V> {
///     # marker: ::std::marker::PhantomData<(K, V)>,
///     // fields omitted
/// }
///
/// impl<K, V> HashMap<K, V> {
///     pub fn insert(&self, key: K, value: V) -> Option<V>
///     where K: Hash + Eq
///     {
///         # unimplemented!()
///         // ...
///     }
///
///     pub fn get<Q>(&self, k: &Q) -> Option<&V>
///     where
///         K: Borrow<Q>,
///         Q: Hash + Eq + ?Sized
///     {
///         # unimplemented!()
///         // ...
///     }
/// }
/// ```
///
/// The entire hash map is generic over a key type `K`. Because these keys
/// are stored with the hash map, this type has to own the key’s data.
/// When inserting a key-value pair, the map is given such a `K` and needs
/// to find the correct hash bucket and check if the key is already present
/// based on that `K`. It therefore requires `K: Hash + Eq`.
///
/// When searching for a value in the map, however, having to provide a
/// reference to a `K` as the key to search for would require to always
/// create such an owned value. For string keys, this would mean a `String`
/// value needs to be created just for the search for cases where only a
/// `str` is available.
///
/// Instead, the `get` method is generic over the type of the underlying key
/// data, called `Q` in the method signature above. It states that `K`
/// borrows as a `Q` by requiring that `K: Borrow<Q>`. By additionally
/// requiring `Q: Hash + Eq`, it signals the requirement that `K` and `Q`
/// have implementations of the `Hash` and `Eq` traits that produce identical
/// results.
///
/// The implementation of `get` relies in particular on identical
/// implementations of `Hash` by determining the key’s hash bucket by calling
/// `Hash::hash` on the `Q` value even though it inserted the key based on
/// the hash value calculated from the `K` value.
///
/// As a consequence, the hash map breaks if a `K` wrapping a `Q` value
/// produces a different hash than `Q`. For instance, imagine you have a
/// type that wraps a string but compares ASCII letters ignoring their case:
///
/// ```
/// pub struct CaseInsensitiveString(String);
///
/// impl PartialEq for CaseInsensitiveString {
///     fn eq(&self, other: &Self) -> bool {
///         self.0.eq_ignore_ascii_case(&other.0)
///     }
/// }
///
/// impl Eq for CaseInsensitiveString { }
/// ```
///
/// Because two equal values need to produce the same hash value, the
/// implementation of `Hash` needs to ignore ASCII case, too:
///
/// ```
/// # use std::hash::{Hash, Hasher};
/// # pub struct CaseInsensitiveString(String);
/// impl Hash for CaseInsensitiveString {
///     fn hash<H: Hasher>(&self, state: &mut H) {
///         for c in self.0.as_bytes() {
///             c.to_ascii_lowercase().hash(state)
///         }
///     }
/// }
/// ```
///
/// Can `CaseInsensitiveString` implement `Borrow<str>`? It certainly can
/// provide a reference to a string slice via its contained owned string.
/// But because its `Hash` implementation differs, it behaves differently
/// from `str` and therefore must not, in fact, implement `Borrow<str>`.
/// If it wants to allow others access to the underlying `str`, it can do
/// that via `AsRef<str>` which doesn’t carry any extra requirements.
///
/// [`Hash`]: crate::hash::Hash
/// [`HashMap<K, V>`]: ../../std/collections/struct.HashMap.html
/// [`String`]: ../../std/string/struct.String.html
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_diagnostic_item = "Borrow"]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
pub const trait Borrow<Borrowed: ?Sized> {
    /// Immutably borrows from an owned value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Borrow;
    ///
    /// fn check<T: Borrow<str>>(s: T) {
    ///     assert_eq!("Hello", s.borrow());
    /// }
    ///
    /// let s = "Hello".to_string();
    ///
    /// check(s);
    ///
    /// let s = "Hello";
    ///
    /// check(s);
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    fn borrow(&self) -> &Borrowed;
}

/// A trait for mutably borrowing data.
///
/// As a companion to [`Borrow<T>`] this trait allows a type to borrow as
/// an underlying type by providing a mutable reference. See [`Borrow<T>`]
/// for more information on borrowing as another type.
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_diagnostic_item = "BorrowMut"]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
pub const trait BorrowMut<Borrowed: ?Sized>: [const] Borrow<Borrowed> {
    /// Mutably borrows from an owned value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::BorrowMut;
    ///
    /// fn check<T: BorrowMut<[i32]>>(mut v: T) {
    ///     assert_eq!(&mut [1, 2, 3], v.borrow_mut());
    /// }
    ///
    /// let v = vec![1, 2, 3];
    ///
    /// check(v);
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    fn borrow_mut(&mut self) -> &mut Borrowed;
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T: ?Sized> Borrow<T> for T {
    #[rustc_diagnostic_item = "noop_method_borrow"]
    fn borrow(&self) -> &T {
        self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T: ?Sized> BorrowMut<T> for T {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T: ?Sized> Borrow<T> for &T {
    fn borrow(&self) -> &T {
        self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T: ?Sized> Borrow<T> for &mut T {
    fn borrow(&self) -> &T {
        self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_convert", issue = "143773")]
const impl<T: ?Sized> BorrowMut<T> for &mut T {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

/// A generalization of `Clone` to borrowed data.
///
/// Some types make it possible to go from borrowed to owned, usually by
/// implementing the `Clone` trait. But `Clone` works only for going from `&T`
/// to `T`. The `ToOwned` trait generalizes `Clone` to construct owned data
/// from any borrow of a given type.
#[rustc_diagnostic_item = "ToOwned"]
#[unstable(feature = "core_to_owned", issue = "none")]
pub trait ToOwned {
    /// The resulting type after obtaining ownership.
    #[stable(feature = "rust1", since = "1.0.0")]
    type Owned: Borrow<Self>;

    /// Creates owned data from borrowed data, usually by cloning.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "a";
    /// let ss: String = s.to_owned();
    ///
    /// let v: &[i32] = &[1, 2];
    /// let vv: Vec<i32> = v.to_owned();
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[must_use = "cloning is often expensive and is not expected to have side effects"]
    #[rustc_diagnostic_item = "to_owned_method"]
    fn to_owned(&self) -> Self::Owned;

    /// Uses borrowed data to replace owned data, usually by cloning.
    ///
    /// This is borrow-generalized version of [`Clone::clone_from`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let mut s: String = String::new();
    /// "hello".clone_into(&mut s);
    ///
    /// let mut v: Vec<i32> = Vec::new();
    /// [1, 2][..].clone_into(&mut v);
    /// ```
    #[stable(feature = "toowned_clone_into", since = "1.63.0")]
    fn clone_into(&self, target: &mut Self::Owned) {
        *target = self.to_owned();
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T> ToOwned for T
where
    T: Clone,
{
    type Owned = T;
    fn to_owned(&self) -> T {
        self.clone()
    }

    fn clone_into(&self, target: &mut T) {
        target.clone_from(self);
    }
}

/// A clone-on-write smart pointer.
///
/// The type `Cow` is a smart pointer providing clone-on-write functionality: it
/// can enclose and provide immutable access to borrowed data, and clone the
/// data lazily when mutation or ownership is required. The type is designed to
/// work with general borrowed data via the `Borrow` trait.
///
/// `Cow` implements `Deref`, which means that you can call
/// non-mutating methods directly on the data it encloses. If mutation
/// is desired, `to_mut` will obtain a mutable reference to an owned
/// value, cloning if necessary.
///
/// If you need reference-counting pointers, note that
/// [`Rc::make_mut`][crate::rc::Rc::make_mut] and
/// [`Arc::make_mut`][crate::sync::Arc::make_mut] can provide clone-on-write
/// functionality as well.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
///
/// fn abs_all(input: &mut Cow<'_, [i32]>) {
///     for i in 0..input.len() {
///         let v = input[i];
///         if v < 0 {
///             // Clones into a vector if not already owned.
///             input.to_mut()[i] = -v;
///         }
///     }
/// }
///
/// // No clone occurs because `input` doesn't need to be mutated.
/// let slice = [0, 1, 2];
/// let mut input = Cow::from(&slice[..]);
/// abs_all(&mut input);
///
/// // Clone occurs because `input` needs to be mutated.
/// let slice = [-1, 0, 1];
/// let mut input = Cow::from(&slice[..]);
/// abs_all(&mut input);
///
/// // No clone occurs because `input` is already owned.
/// let mut input = Cow::from(vec![-1, 0, 1]);
/// abs_all(&mut input);
/// ```
///
/// Another example showing how to keep `Cow` in a struct:
///
/// ```
/// use std::borrow::Cow;
///
/// struct Items<'a, X> where [X]: ToOwned<Owned = Vec<X>> {
///     values: Cow<'a, [X]>,
/// }
///
/// impl<'a, X: Clone + 'a> Items<'a, X> where [X]: ToOwned<Owned = Vec<X>> {
///     fn new(v: Cow<'a, [X]>) -> Self {
///         Items { values: v }
///     }
/// }
///
/// // Creates a container from borrowed values of a slice
/// let readonly = [1, 2];
/// let borrowed = Items::new((&readonly[..]).into());
/// match borrowed {
///     Items { values: Cow::Borrowed(b) } => println!("borrowed {b:?}"),
///     _ => panic!("expect borrowed value"),
/// }
///
/// let mut clone_on_write = borrowed;
/// // Mutates the data from slice into owned vec and pushes a new value on top
/// clone_on_write.values.to_mut().push(3);
/// println!("clone_on_write = {:?}", clone_on_write.values);
///
/// // The data was mutated. Let's check it out.
/// match clone_on_write {
///     Items { values: Cow::Owned(_) } => println!("clone_on_write contains owned data"),
///     _ => panic!("expect owned data"),
/// }
/// ```
#[unstable(feature = "core_to_owned", issue = "none")]
#[rustc_diagnostic_item = "Cow"]
pub enum Cow<'a, B: ?Sized + 'a>
where
    B: ToOwned,
{
    /// Borrowed data.
    #[stable(feature = "rust1", since = "1.0.0")]
    Borrowed(#[stable(feature = "rust1", since = "1.0.0")] &'a B),

    /// Owned data.
    #[stable(feature = "rust1", since = "1.0.0")]
    Owned(#[stable(feature = "rust1", since = "1.0.0")] <B as ToOwned>::Owned),
}

// FIXME(inference): const bounds removed due to inference regressions found by crater;
//   see https://github.com/rust-lang/rust/issues/147964
// #[rustc_const_unstable(feature = "const_convert", issue = "143773")]
#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, B: ?Sized + ToOwned> Borrow<B> for Cow<'a, B>
// where
//     B::Owned: [const] Borrow<B>,
{
    fn borrow(&self) -> &B {
        &**self
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized + ToOwned> Clone for Cow<'_, B> {
    fn clone(&self) -> Self {
        match *self {
            Cow::Borrowed(b) => Cow::Borrowed(b),
            Cow::Owned(ref o) => {
                let b: &B = o.borrow();
                Cow::Owned(b.to_owned())
            }
        }
    }

    fn clone_from(&mut self, source: &Self) {
        match (self, source) {
            (&mut Cow::Owned(ref mut dest), &Cow::Owned(ref o)) => o.borrow().clone_into(dest),
            (t, s) => *t = s.clone(),
        }
    }
}

impl<B: ?Sized + ToOwned> Cow<'_, B> {
    /// Returns true if the data is borrowed, i.e. if `to_mut` would require additional work.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `Cow::is_borrowed(&c)` instead of `c.is_borrowed()`. This is so
    /// that there is no conflict with a method on the inner type.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(cow_is_borrowed)]
    /// use std::borrow::Cow;
    ///
    /// let cow = Cow::Borrowed("moo");
    /// assert!(Cow::is_borrowed(&cow));
    ///
    /// let bull: Cow<'_, str> = Cow::Owned("...moo?".to_string());
    /// assert!(!Cow::is_borrowed(&bull));
    /// ```
    #[unstable(feature = "cow_is_borrowed", issue = "65143")]
    pub const fn is_borrowed(c: &Self) -> bool {
        match *c {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    /// Returns true if the data is owned, i.e. if `to_mut` would be a no-op.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `Cow::is_owned(&c)` instead of `c.is_owned()`. This is so that
    /// there is no conflict with a method on the inner type.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(cow_is_borrowed)]
    /// use std::borrow::Cow;
    ///
    /// let cow: Cow<'_, str> = Cow::Owned("moo".to_string());
    /// assert!(Cow::is_owned(&cow));
    ///
    /// let bull = Cow::Borrowed("...moo?");
    /// assert!(!Cow::is_owned(&bull));
    /// ```
    #[unstable(feature = "cow_is_borrowed", issue = "65143")]
    pub const fn is_owned(c: &Self) -> bool {
        !Cow::is_borrowed(c)
    }

    /// Acquires a mutable reference to the owned form of the data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let mut cow = Cow::Borrowed("foo");
    /// cow.to_mut().make_ascii_uppercase();
    ///
    /// assert_eq!(
    ///   cow,
    ///   Cow::Owned(String::from("FOO")) as Cow<'_, str>
    /// );
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn to_mut(&mut self) -> &mut <B as ToOwned>::Owned {
        match *self {
            Cow::Borrowed(borrowed) => {
                *self = Cow::Owned(borrowed.to_owned());
                match *self {
                    Cow::Borrowed(..) => unreachable!(),
                    Cow::Owned(ref mut owned) => owned,
                }
            }
            Cow::Owned(ref mut owned) => owned,
        }
    }

    /// Extracts the owned data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// Calling `into_owned` on a `Cow::Borrowed` returns a clone of the borrowed data:
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// assert_eq!(
    ///   cow.into_owned(),
    ///   String::from(s)
    /// );
    /// ```
    ///
    /// Calling `into_owned` on a `Cow::Owned` returns the owned data. The data is moved out of the
    /// `Cow` without being cloned.
    ///
    /// ```
    /// use std::borrow::Cow;
    ///
    /// let s = "Hello world!";
    /// let cow: Cow<'_, str> = Cow::Owned(String::from(s));
    ///
    /// assert_eq!(
    ///   cow.into_owned(),
    ///   String::from(s)
    /// );
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn into_owned(self) -> <B as ToOwned>::Owned {
        match self {
            Cow::Borrowed(borrowed) => borrowed.to_owned(),
            Cow::Owned(owned) => owned,
        }
    }
}

// FIXME(inference): const bounds removed due to inference regressions found by crater;
//   see https://github.com/rust-lang/rust/issues/147964
// #[rustc_const_unstable(feature = "const_convert", issue = "143773")]
#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized + ToOwned> Deref for Cow<'_, B>
// where
//     B::Owned: [const] Borrow<B>,
{
    type Target = B;

    fn deref(&self) -> &B {
        match *self {
            Cow::Borrowed(borrowed) => borrowed,
            Cow::Owned(ref owned) => owned.borrow(),
        }
    }
}

// `Cow<'_, T>` can only implement `DerefPure` if `<T::Owned as Borrow<T>>` (and `BorrowMut<T>`) is trusted.
// For now, we restrict `DerefPure for Cow<T>` to `T: Sized` (`T as Borrow<T>` is trusted),
// `str` (`String as Borrow<str>` is trusted) and `[T]` (`Vec<T> as Borrow<[T]>` is trusted).
// In the future, a `BorrowPure<T>` trait analogous to `DerefPure` might generalize this.
#[unstable(feature = "deref_pure_trait", issue = "87121")]
unsafe impl<T: Clone> DerefPure for Cow<'_, T> {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized> Eq for Cow<'_, B> where B: Eq + ToOwned {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized> Ord for Cow<'_, B>
where
    B: Ord + ToOwned,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, 'b, B: ?Sized, C: ?Sized> PartialEq<Cow<'b, C>> for Cow<'a, B>
where
    B: PartialEq<C> + ToOwned,
    C: ToOwned,
{
    #[inline]
    fn eq(&self, other: &Cow<'b, C>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, B: ?Sized> PartialOrd for Cow<'a, B>
where
    B: PartialOrd + ToOwned,
{
    #[inline]
    fn partial_cmp(&self, other: &Cow<'a, B>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized> fmt::Debug for Cow<'_, B>
where
    B: fmt::Debug + ToOwned<Owned: fmt::Debug>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cow::Borrowed(ref b) => fmt::Debug::fmt(b, f),
            Cow::Owned(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized> fmt::Display for Cow<'_, B>
where
    B: fmt::Display + ToOwned<Owned: fmt::Display>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cow::Borrowed(ref b) => fmt::Display::fmt(b, f),
            Cow::Owned(ref o) => fmt::Display::fmt(o, f),
        }
    }
}

#[stable(feature = "default", since = "1.11.0")]
impl<B: ?Sized> Default for Cow<'_, B>
where
    B: ToOwned<Owned: Default>,
{
    /// Creates an owned Cow<'a, B> with the default value for the contained owned value.
    fn default() -> Self {
        Cow::Owned(<B as ToOwned>::Owned::default())
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized> Hash for Cow<'_, B>
where
    B: Hash + ToOwned,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

// FIXME(inference): const bounds removed due to inference regressions found by crater;
//   see https://github.com/rust-lang/rust/issues/147964
// #[rustc_const_unstable(feature = "const_convert", issue = "143773")]
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized + ToOwned> AsRef<T> for Cow<'_, T>
// where
//     T::Owned: [const] Borrow<T>,
{
    fn as_ref(&self) -> &T {
        self
    }
}
