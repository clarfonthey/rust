macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty, $($constraints:tt)*) => {
        #[stable(feature = "vec_deque_partial_eq_slice", since = "1.17.0")]
        #[rustc_const_unstable(feature = "const_cmp", issue = "143800")]
        impl<T, U, A: Allocator, $($vars)*> const PartialEq<$rhs> for $lhs
        where
            T: ~const PartialEq<U>,
            $($constraints)*
        {
            fn eq(&self, other: &$rhs) -> bool {
                if self.len() != other.len() {
                    return false;
                }
                let (sa, sb) = self.as_slices();
                let (oa, ob) = other.split_at(sa.len());
                sa == oa && sb == ob
            }
        }
    }
}
