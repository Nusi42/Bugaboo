//! The seal of cursors and `ETag`s (PERF-02, PERF-03), kept beside the use cases in `bb-app` so the
//! cursors of the API and of the tools of AI apps share one key and one rule.

pub(crate) use bb_app::seal::process;
