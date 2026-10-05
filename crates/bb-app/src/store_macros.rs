//! The two macros that write the reads and the writes of a store port from one line each, shared
//! by the ports of `bb-app` (`AuthStore`, `PolicyStore`).

/// The reads of the port: `fn name(args) -> Type` becomes a method returning
/// `Result<Type, Self::Error>`.
macro_rules! reads {
    ($($(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)+) => {
        $($(#[$meta])*
        fn $name(&self $(, $arg: $ty)*)
            -> impl Future<Output = Result<$ret, Self::Error>> + Send;)+
    };
}

/// The writes of the port: the same, inside the transaction of a change. They are associated
/// functions, not methods: the closure of a change is called with the transaction and nothing
/// else, so it cannot hold on to the store.
macro_rules! writes {
    ($($(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)+) => {
        $($(#[$meta])*
        fn $name(transaction: &mut Self::Transaction $(, $arg: $ty)*)
            -> impl Future<Output = Result<$ret, Self::Error>> + Send;)+
    };
}

pub(crate) use reads;
pub(crate) use writes;
