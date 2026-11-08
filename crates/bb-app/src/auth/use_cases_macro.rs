//! The macros that list the use cases of [`AuthApi`](super::AuthApi) once (`use_cases!` in `mod.rs`).

/// Declares [`AuthApi`] and implements it for [`AuthServices`]: one line per use case, the
/// name of the function above, its input and its output.
macro_rules! use_cases {
    ($($(#[$meta:meta])* $name:ident($input:ty) -> $output:ty $(= $via:ident)?;)+) => {
        /// Every use case of [`AuthApi`] by the name of the function that does it, with the name
        /// of its input type: what tests read to make sure none is left out (a use case whose
        /// input ends in `Query` only reads). Two requests may share one function (`= via`).
        pub const USE_CASES: &[(&str, &str)] =
            &[$((function_name!($name $($via)?), stringify!($input))),+];

        /// Every use case the HTTP layer calls, with the store behind it erased.
        pub trait AuthApi: Send + Sync {
            /// Whether `presented` (the `x-csrf-token` header) is the token of the session
            /// behind `session_token` (the cookie).
            fn csrf_matches(&self, session_token: &str, presented: &str) -> bool;

            $($(#[$meta])*
            fn $name(&self, request: RequestInfo, input: $input) -> Answer<'_, $output>;)+
        }

        impl<S: AuthStore + 'static> AuthApi for AuthServices<S> {
            fn csrf_matches(&self, session_token: &str, presented: &str) -> bool {
                Self::csrf_matches(self, session_token, presented)
            }

            $(fn $name(&self, request: RequestInfo, input: $input) -> Answer<'_, $output>
            {
                Box::pin(async move {
                    let ctx = self.ctx(request);
                    // An input that does not end in `Query` changes something: an agent's limits
                    // count it (`Ctx::count_agent_change`).
                    if !stringify!($input).ends_with("Query") {
                        ctx.expect_change();
                    }
                    match function!($name $($via)?)(&ctx, input).await {
                        // The one place where a stale version becomes the structured answer.
                        Err(AppError::Stale(stale)) => Err(ctx.conflict_of(&stale).await),
                        answer => {
                            // A change that was made counts for an agent; one that was refused does not.
                            if answer.is_ok() {
                                ctx.settle_agent_change();
                            }
                            answer
                        }
                    }
                })
            })+
        }
    };
}

/// The name of the function that does a use case: its own name, or the one it shares.
macro_rules! function_name {
    ($name:ident) => {
        stringify!($name)
    };
    ($name:ident $via:ident) => {
        stringify!($via)
    };
}

/// The function that does a use case, as a path to call.
macro_rules! function {
    ($name:ident) => {
        $name
    };
    ($name:ident $via:ident) => {
        $via
    };
}
