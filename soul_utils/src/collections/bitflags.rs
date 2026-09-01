#[macro_export]
macro_rules! bitflags {
    (
        $(#[$struct_attrs:meta])*
        $vis:vis struct $Name:ident : $T:ty {
            $(
                $(#[$attr:meta])*
                $Flag:ident = $value:expr,
            )*
        }
    ) => {
        $(#[$struct_attrs])*
        $vis struct $Name($T);

        impl Default for $Name {
            fn default() -> Self {
                Self::empty()
            }
        }

        #[allow(unused)]
        impl $Name {
            $(
                $(#[$attr])*
                pub const $Flag: Self = Self($value);
            )*

            #[inline]
            pub const fn empty() -> Self {
                Self(0)
            }

            #[inline]
            pub const fn all() -> Self {
                Self($($value)|*)
            }

            #[inline]
            pub const fn bits(&self) -> $T {
                self.0
            }

            #[inline]
            pub const fn from_bits(bits: $T) -> Option<Self> {
                // Optional: validate bits if you want
                Some(Self(bits))
            }

            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }

            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                (self.0 & other.0) != 0
            }

            #[inline]
            pub const fn union(&self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            #[inline]
            pub const fn difference(&self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            #[inline]
            pub const fn symmetric_difference(&self, other: Self) -> Self {
                Self(self.0 ^ other.0)
            }

            #[inline]
            pub const fn complement(&self) -> Self {
                Self(!self.0)
            }
        }

        impl core::ops::BitOr for $Name {
            type Output = Self;
            #[inline]
            fn bitor(self, rhs: Self) -> Self::Output {
                self.union(rhs)
            }
        }

        impl core::ops::BitOrAssign for $Name {
            #[inline]
            fn bitor_assign(&mut self, rhs: Self) {
                *self = self.union(rhs);
            }
        }

        impl core::ops::BitAnd for $Name {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: Self) -> Self::Output {
                Self(self.0 & rhs.0)
            }
        }

        impl core::ops::BitAndAssign for $Name {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) {
                *self = Self(self.0 & rhs.0);
            }
        }

        impl core::ops::BitXor for $Name {
            type Output = Self;
            #[inline]
            fn bitxor(self, rhs: Self) -> Self::Output {
                self.symmetric_difference(rhs)
            }
        }

        impl core::ops::BitXorAssign for $Name {
            #[inline]
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = self.symmetric_difference(rhs);
            }
        }

        impl core::ops::Not for $Name {
            type Output = Self;
            #[inline]
            fn not(self) -> Self::Output {
                self.complement()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    bitflags! {
        pub struct Permissions: u8 {
            READ    = 1 << 0,
            WRITE   = 1 << 1,
            EXECUTE = 1 << 2,
        }
    }

    #[test]
    fn test_basic_bitflag() {
        let r = Permissions::READ;
        let w = Permissions::WRITE;
        let rw = r | w;

        assert!(rw.contains(Permissions::READ));
        assert!(rw.contains(Permissions::WRITE));
        assert!(!rw.contains(Permissions::EXECUTE));

        let rx = Permissions::READ | Permissions::EXECUTE;
        let diff = rw ^ rx; // toggles READ and EXECUTE
        assert!(diff.contains(Permissions::WRITE));
        assert!(diff.contains(Permissions::EXECUTE));
        assert!(!diff.contains(Permissions::READ));
    }
}
