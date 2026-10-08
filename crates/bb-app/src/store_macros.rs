//! The three macros that write the reads and the writes of a store port from one line each, shared
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

/// A read that is needed both on its own and inside a change: `fn name(args) -> Type =
/// name_in;` declares `name_in`, which the store implements once on an open transaction, and
/// writes `name` from it (a snapshot of the store, [`ChangeStore::read_snapshot`]). The store and
/// the in-memory fake each write the read once.
macro_rules! queries {
    ($($(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty = $name_in:ident;)+) => {
        $($(#[$meta])*
        fn $name(&self $(, $arg: $ty)*)
            -> impl Future<Output = Result<$ret, Self::Error>> + Send {
            self.read_snapshot(move |transaction| Box::pin(Self::$name_in(transaction $(, $arg)*)))
        }

        #[doc = concat!(
            "The same as [`Self::", stringify!($name), "`], on a transaction that is open: ",
            "inside a change it sees what the change wrote."
        )]
        fn $name_in(transaction: &mut Self::Transaction $(, $arg: $ty)*)
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

pub(crate) use queries;
pub(crate) use reads;
pub(crate) use writes;
