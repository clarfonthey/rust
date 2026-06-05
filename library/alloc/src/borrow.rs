//! A module for working with borrowed data.

#![stable(feature = "rust1", since = "1.0.0")]

#[stable(feature = "rust1", since = "1.0.0")]
pub use core::borrow::{Borrow, BorrowMut, Cow, ToOwned};
use core::ops::DerefPure;
#[cfg(not(no_global_oom_handling))]
use core::ops::{Add, AddAssign};

#[cfg(not(no_global_oom_handling))]
use crate::string::String;

#[cfg(not(no_global_oom_handling))]
#[unstable(feature = "deref_pure_trait", issue = "87121")]
unsafe impl DerefPure for Cow<'_, str> {}
#[cfg(not(no_global_oom_handling))]
#[unstable(feature = "deref_pure_trait", issue = "87121")]
unsafe impl<T: Clone> DerefPure for Cow<'_, [T]> {}

#[cfg(not(no_global_oom_handling))]
#[stable(feature = "cow_add", since = "1.14.0")]
impl<'a> Add<&'a str> for Cow<'a, str> {
    type Output = Cow<'a, str>;

    #[inline]
    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}

#[cfg(not(no_global_oom_handling))]
#[stable(feature = "cow_add", since = "1.14.0")]
impl<'a> Add<Cow<'a, str>> for Cow<'a, str> {
    type Output = Cow<'a, str>;

    #[inline]
    fn add(mut self, rhs: Cow<'a, str>) -> Self::Output {
        self += rhs;
        self
    }
}

#[cfg(not(no_global_oom_handling))]
#[stable(feature = "cow_add", since = "1.14.0")]
impl<'a> AddAssign<&'a str> for Cow<'a, str> {
    fn add_assign(&mut self, rhs: &'a str) {
        if self.is_empty() {
            *self = Cow::Borrowed(rhs)
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = *self {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *self = Cow::Owned(s);
            }
            self.to_mut().push_str(rhs);
        }
    }
}

#[cfg(not(no_global_oom_handling))]
#[stable(feature = "cow_add", since = "1.14.0")]
impl<'a> AddAssign<Cow<'a, str>> for Cow<'a, str> {
    fn add_assign(&mut self, rhs: Cow<'a, str>) {
        if self.is_empty() {
            *self = rhs
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = *self {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *self = Cow::Owned(s);
            }
            self.to_mut().push_str(&rhs);
        }
    }
}
