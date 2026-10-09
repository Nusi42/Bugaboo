//! Plain values: a type that is a value and not a view with fields that a mask must cut.
//!
//! `#[derive(Masked)]` asks for it of every member marked `open` (nothing hidden in it, everybody
//! who sees the view sees it). A member that holds another view, with fields of the catalogue in
//! it, is not plain, so marking it `open` does not compile: it has to be `nested` and masked by
//! its own mask (SEC-04, M2-F21). Words and IDs are plain; so are numbers, text, moments and
//! lists and options of plain values.

use jiff::Timestamp;

/// A value that holds no field of the catalogue under a mask of its own.
#[diagnostic::on_unimplemented(
    message = "`{Self}` may hold fields that a mask has to cut, so it cannot be an `open` member",
    label = "mark this member `#[masked(nested)]`, or make the type a plain value",
    note = "an `open` member of a view is shown to everybody who sees the view (SEC-04, M2-F21)"
)]
pub trait Plain {}

macro_rules! plain {
    ($($ty:ty),+ $(,)?) => { $(impl Plain for $ty {})+ };
}

plain!(
    bool, u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, char, String, str,
    Timestamp,
);

impl<T: Plain> Plain for Option<T> {}
impl<T: Plain> Plain for Vec<T> {}
impl<T: Plain + ?Sized> Plain for Box<T> {}
