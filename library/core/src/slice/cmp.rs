//! Comparison traits for `[T]`.

use super::{from_raw_parts, memchr};
use crate::ascii;
use crate::cmp::{self, BytewiseEq, Ordering};
use crate::convert::Infallible;
use crate::intrinsics::compare_bytes;
use crate::marker::Destruct;
use crate::num::NonZero;
use crate::ops::ControlFlow;

#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<T, U> const PartialEq<[U]> for [T]
where
    T: ~const PartialEq<U>,
{
    fn eq(&self, other: &[U]) -> bool {
        SlicePartialEq::equal(self, other)
    }

    fn ne(&self, other: &[U]) -> bool {
        SlicePartialEq::not_equal(self, other)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: Eq> Eq for [T] {}

/// Implements comparison of slices [lexicographically](Ord#lexicographical-comparison).
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<T: ~const Ord> const Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        SliceOrd::compare(self, other)
    }
}

#[inline]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
const fn as_underlying(x: ControlFlow<bool>) -> u8 {
    // SAFETY: This will only compile if `bool` and `ControlFlow<bool>` have the same
    // size (which isn't guaranteed but this is libcore). Because they have the same
    // size, it's a niched implementation, which in one byte means there can't be
    // any uninitialized memory. The callers then only check for `0` or `1` from this,
    // which must necessarily match the `Break` variant, and we're fine no matter
    // what ends up getting picked as the value representing `Continue(())`.
    unsafe { crate::mem::transmute(x) }
}

/// Implements comparison of slices [lexicographically](Ord#lexicographical-comparison).
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<T: ~const PartialOrd> const PartialOrd for [T] {
    #[inline]
    fn partial_cmp(&self, other: &[T]) -> Option<Ordering> {
        SlicePartialOrd::partial_compare(self, other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        // This is certainly not the obvious way to implement these methods.
        // Unfortunately, using anything that looks at the discriminant means that
        // LLVM sees a check for `2` (aka `ControlFlow<bool>::Continue(())`) and
        // gets very distracted by that, ending up generating extraneous code.
        // This should be changed to something simpler once either LLVM is smarter,
        // see <https://github.com/llvm/llvm-project/issues/132678>, or we generate
        // niche discriminant checks in a way that doesn't trigger it.

        as_underlying(self.__chaining_lt(other)) == 1
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        as_underlying(self.__chaining_le(other)) != 0
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        as_underlying(self.__chaining_gt(other)) == 1
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        as_underlying(self.__chaining_ge(other)) != 0
    }
    #[inline]
    fn __chaining_lt(&self, other: &Self) -> ControlFlow<bool> {
        SliceChain::chaining_lt(self, other)
    }
    #[inline]
    fn __chaining_le(&self, other: &Self) -> ControlFlow<bool> {
        SliceChain::chaining_le(self, other)
    }
    #[inline]
    fn __chaining_gt(&self, other: &Self) -> ControlFlow<bool> {
        SliceChain::chaining_gt(self, other)
    }
    #[inline]
    fn __chaining_ge(&self, other: &Self) -> ControlFlow<bool> {
        SliceChain::chaining_ge(self, other)
    }
}

#[doc(hidden)]
// intermediate trait for specialization of slice's PartialEq
#[const_trait]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
trait SlicePartialEq<B> {
    fn equal(&self, other: &[B]) -> bool;

    fn not_equal(&self, other: &[B]) -> bool {
        !self.equal(other)
    }
}

// Generic slice equality
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A, B> const SlicePartialEq<B> for [A]
where
    A: ~const PartialEq<B>,
{
    default fn equal(&self, other: &[B]) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // Implemented as explicit indexing rather
        // than zipped iterators for performance reasons.
        // See PR https://github.com/rust-lang/rust/pull/116846
        // FIXME(const_hack): make this a `for idx in 0..self.len()` loop.
        let mut idx = 0;
        while idx < self.len() {
            // bound checks are optimized away
            if self[idx] != other[idx] {
                return false;
            }
            idx += 1;
        }

        true
    }
}

// When each element can be compared byte-wise, we can compare all the bytes
// from the whole size in one call to the intrinsics.
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A, B> const SlicePartialEq<B> for [A]
where
    A: ~const BytewiseEq<B>,
{
    fn equal(&self, other: &[B]) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // SAFETY: `self` and `other` are references and are thus guaranteed to be valid.
        // The two slices have been checked to have the same size above.
        unsafe {
            let size = size_of_val(self);
            compare_bytes(self.as_ptr() as *const u8, other.as_ptr() as *const u8, size) == 0
        }
    }
}

#[doc(hidden)]
#[const_trait]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
// intermediate trait for specialization of slice's PartialOrd
trait SlicePartialOrd: Sized {
    fn partial_compare(left: &[Self], right: &[Self]) -> Option<Ordering>;
}

#[doc(hidden)]
#[const_trait]
// intermediate trait for specialization of slice's PartialOrd chaining methods
trait SliceChain: Sized {
    fn chaining_lt(left: &[Self], right: &[Self]) -> ControlFlow<bool>;
    fn chaining_le(left: &[Self], right: &[Self]) -> ControlFlow<bool>;
    fn chaining_gt(left: &[Self], right: &[Self]) -> ControlFlow<bool>;
    fn chaining_ge(left: &[Self], right: &[Self]) -> ControlFlow<bool>;
}

type AlwaysBreak<B> = ControlFlow<B, crate::convert::Infallible>;

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const PartialOrd> const SlicePartialOrd for A {
    default fn partial_compare(left: &[A], right: &[A]) -> Option<Ordering> {
        #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
        const fn elem_chain<A: ~const PartialOrd>(a: &A, b: &A) -> ControlFlow<Option<Ordering>> {
            match PartialOrd::partial_cmp(a, b) {
                Some(Ordering::Equal) => ControlFlow::Continue(()),
                non_eq => ControlFlow::Break(non_eq),
            }
        }
        #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
        const fn len_chain(a: &usize, b: &usize) -> ControlFlow<Option<Ordering>, Infallible> {
            ControlFlow::Break(usize::partial_cmp(a, b))
        }
        let AlwaysBreak::Break(b) = chaining_impl(left, right, elem_chain, len_chain);
        b
    }
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const PartialOrd> const SliceChain for A {
    default fn chaining_lt(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        chaining_impl(left, right, PartialOrd::__chaining_lt, usize::__chaining_lt)
    }
    default fn chaining_le(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        chaining_impl(left, right, PartialOrd::__chaining_le, usize::__chaining_le)
    }
    default fn chaining_gt(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        chaining_impl(left, right, PartialOrd::__chaining_gt, usize::__chaining_gt)
    }
    default fn chaining_ge(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        chaining_impl(left, right, PartialOrd::__chaining_ge, usize::__chaining_ge)
    }
}

#[inline]
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
const fn chaining_impl<'l, 'r, A: ~const PartialOrd, B, C>(
    left: &'l [A],
    right: &'r [A],
    elem_chain: impl ~const Fn(&'l A, &'r A) -> ControlFlow<B> + ~const Destruct,
    len_chain: impl ~const Destruct + for<'a> ~const FnOnce(&'a usize, &'a usize) -> ControlFlow<B, C>,
) -> ControlFlow<B, C> {
    let l = cmp::min(left.len(), right.len());

    // Slice to the loop iteration range to enable bound check
    // elimination in the compiler
    let lhs = &left[..l];
    let rhs = &right[..l];

    let mut i = 0;
    while i < l {
        elem_chain(&lhs[i], &rhs[i])?;
        i += 1;
    }

    len_chain(&left.len(), &right.len())
}

// This is the impl that we would like to have. Unfortunately it's not sound.
// See `partial_ord_slice.rs`.
/*
impl<A> SlicePartialOrd for A
where
    A: Ord,
{
    default fn partial_compare(left: &[A], right: &[A]) -> Option<Ordering> {
        Some(SliceOrd::compare(left, right))
    }
}
*/

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const AlwaysApplicableOrd> const SlicePartialOrd for A {
    fn partial_compare(left: &[A], right: &[A]) -> Option<Ordering> {
        Some(SliceOrd::compare(left, right))
    }
}

#[rustc_specialization_trait]
#[const_trait]
trait AlwaysApplicableOrd: ~const SliceOrd + ~const Ord {}

macro_rules! always_applicable_ord {
    ($([$($p:tt)*] $t:ty $(, $const:tt)?;)*) => {
        $(impl<$($p)*> $($const)? AlwaysApplicableOrd for $t {})*
    }
}

always_applicable_ord! {
    [] u8, const; [] u16, const; [] u32, const; [] u64, const; [] u128, const; [] usize, const;
    [] i8, const; [] i16, const; [] i32, const; [] i64, const; [] i128, const; [] isize, const;
    [] bool, const; [] char, const;
    [T: ?Sized] *const T; [T: ?Sized] *mut T;
    [T: ~const AlwaysApplicableOrd] &T, const;
    [T: ~const AlwaysApplicableOrd] &mut T, const;
    [T: ~const AlwaysApplicableOrd] Option<T>, const;
}

#[doc(hidden)]
// intermediate trait for specialization of slice's Ord
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
#[const_trait]
trait SliceOrd: Sized {
    fn compare(left: &[Self], right: &[Self]) -> Ordering;
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const Ord> const SliceOrd for A {
    default fn compare(left: &[Self], right: &[Self]) -> Ordering {
        #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
        const fn elem_chain<A: ~const Ord>(a: &A, b: &A) -> ControlFlow<Ordering> {
            match Ord::cmp(a, b) {
                Ordering::Equal => ControlFlow::Continue(()),
                non_eq => ControlFlow::Break(non_eq),
            }
        }
        #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
        const fn len_chain(a: &usize, b: &usize) -> ControlFlow<Ordering, Infallible> {
            ControlFlow::Break(usize::cmp(a, b))
        }
        let AlwaysBreak::Break(b) = chaining_impl(left, right, elem_chain, len_chain);
        b
    }
}

/// Marks that a type should be treated as an unsigned byte for comparisons.
///
/// # Safety
/// * The type must be readable as an `u8`, meaning it has to have the same
///   layout as `u8` and always be initialized.
/// * For every `x` and `y` of this type, `Ord(x, y)` must return the same
///   value as `Ord::cmp(transmute::<_, u8>(x), transmute::<_, u8>(y))`.
#[rustc_specialization_trait]
unsafe trait UnsignedBytewiseOrd: Ord {}

unsafe impl UnsignedBytewiseOrd for bool {}
unsafe impl UnsignedBytewiseOrd for u8 {}
unsafe impl UnsignedBytewiseOrd for NonZero<u8> {}
unsafe impl UnsignedBytewiseOrd for Option<NonZero<u8>> {}
unsafe impl UnsignedBytewiseOrd for ascii::Char {}

// `compare_bytes` compares a sequence of unsigned bytes lexicographically, so
// use it if the requirements for `UnsignedBytewiseOrd` are fulfilled.
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const Ord + UnsignedBytewiseOrd> const SliceOrd for A {
    #[inline]
    fn compare(left: &[Self], right: &[Self]) -> Ordering {
        // Since the length of a slice is always less than or equal to
        // isize::MAX, this never underflows.
        let diff = left.len() as isize - right.len() as isize;
        // This comparison gets optimized away (on x86_64 and ARM) because the
        // subtraction updates flags.
        let len = if left.len() < right.len() { left.len() } else { right.len() };
        let left = left.as_ptr().cast();
        let right = right.as_ptr().cast();
        // SAFETY: `left` and `right` are references and are thus guaranteed to
        // be valid. `UnsignedBytewiseOrd` is only implemented for types that
        // are valid u8s and can be compared the same way. We use the minimum
        // of both lengths which guarantees that both regions are valid for
        // reads in that interval.
        let mut order = unsafe { compare_bytes(left, right, len) as isize };
        if order == 0 {
            order = diff;
        }
        order.cmp(&0)
    }
}

// Don't generate our own chaining loops for `memcmp`-able things either.
#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<A: ~const Ord + UnsignedBytewiseOrd> const SliceChain for A {
    #[inline]
    fn chaining_lt(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        match SliceOrd::compare(left, right) {
            Ordering::Equal => ControlFlow::Continue(()),
            ne => ControlFlow::Break(ne.is_lt()),
        }
    }
    #[inline]
    fn chaining_le(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        match SliceOrd::compare(left, right) {
            Ordering::Equal => ControlFlow::Continue(()),
            ne => ControlFlow::Break(ne.is_le()),
        }
    }
    #[inline]
    fn chaining_gt(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        match SliceOrd::compare(left, right) {
            Ordering::Equal => ControlFlow::Continue(()),
            ne => ControlFlow::Break(ne.is_gt()),
        }
    }
    #[inline]
    fn chaining_ge(left: &[Self], right: &[Self]) -> ControlFlow<bool> {
        match SliceOrd::compare(left, right) {
            Ordering::Equal => ControlFlow::Continue(()),
            ne => ControlFlow::Break(ne.is_ge()),
        }
    }
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
#[const_trait]
pub(super) trait SliceContains: Sized {
    fn slice_contains(&self, x: &[Self]) -> bool;
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl<T> const SliceContains for T
where
    T: ~const PartialEq,
{
    default fn slice_contains(&self, x: &[Self]) -> bool {
        let mut idx = 0;
        while idx < x.len() {
            if x[idx] == *self {
                return true;
            }
            idx += 1;
        }
        false
    }
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl const SliceContains for u8 {
    #[inline]
    fn slice_contains(&self, x: &[Self]) -> bool {
        memchr::memchr(*self, x).is_some()
    }
}

#[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
impl const SliceContains for i8 {
    #[inline]
    fn slice_contains(&self, x: &[Self]) -> bool {
        let byte = *self as u8;
        // SAFETY: `i8` and `u8` have the same memory layout, thus casting `x.as_ptr()`
        // as `*const u8` is safe. The `x.as_ptr()` comes from a reference and is thus guaranteed
        // to be valid for reads for the length of the slice `x.len()`, which cannot be larger
        // than `isize::MAX`. The returned slice is never mutated.
        let bytes: &[u8] = unsafe { from_raw_parts(x.as_ptr() as *const u8, x.len()) };
        memchr::memchr(byte, bytes).is_some()
    }
}

macro_rules! impl_slice_contains {
    ($($t:ty),*) => {
        $(
            #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
            impl const SliceContains for $t {
                #[inline]
                fn slice_contains(&self, arr: &[$t]) -> bool {
                    // Make our LANE_COUNT 4x the normal lane count (aiming for 128 bit vectors).
                    // The compiler will nicely unroll it.
                    const LANE_COUNT: usize = 4 * (128 / (size_of::<$t>() * 8));
                    // SIMD
                    let mut idx = 0;
                    while idx < arr.len() {
                        let mut sub_idx = 0;
                        let mut acc = false;
                        while sub_idx < LANE_COUNT {
                            acc |= arr[idx + sub_idx] == *self;
                            sub_idx += 1;
                        }
                        if acc {
                            return true;
                        }
                        idx += LANE_COUNT;
                    }
                    // Scalar remainder
                    while idx < arr.len() {
                        if arr[idx] == *self {
                            return true;
                        }
                        idx += 1;
                    }
                    false
                }
            }
        )*
    };
}

impl_slice_contains!(u16, u32, u64, i16, i32, i64, f32, f64, usize, isize, char);
