//! `every!`: the list of all variants of a plain enum, checked for completeness by the compiler.

/// Writes `Type::ALL`, the array of the listed variants, and an exhaustive `match` over them: a
/// variant of the enum that is not listed is a compile error, and one that is listed twice is an
/// unreachable pattern (a warning, which the lints deny). The list cannot fall behind the enum.
macro_rules! every {
    ($type:ident: $($variant:ident),+ $(,)?) => {
        impl $type {
            /// Every variant, in the order of the declaration.
            pub const ALL: [Self; [$(stringify!($variant)),+].len()] = [$(Self::$variant),+];
        }

        const _: fn($type) = |value| match value {
            $($type::$variant => ()),+
        };
    };
}
