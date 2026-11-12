//! Own themes (THM-03, THM-04, ADR-0026 update M8): a theme for everyone or a personal one is
//! stored as the token values it changes, checked on every save, and served as a style sheet.

mod store;

pub use store::ThemeStore;
