//! Pure logic for the Logbook. No I/O, no async, no `unsafe`.
//!
//! Aeneas verifies the functions in this crate (spec 7.6), so the code keeps to
//! the Rust subset that Aeneas supports: integers, structs, enums, `while` loops
//! with indices, `Option`, `Vec`, and bytes instead of `String`. No iterators or
//! closures in the verified functions, and no std helpers that Aeneas does not
//! model (`u8::is_ascii_*`, `contains` on ranges, `Vec::is_empty`, `Option::clone`).

// Aeneas reads these comparisons best as written.
#![allow(
    clippy::manual_range_contains,
    clippy::len_zero,
    clippy::manual_is_ascii_check,
    clippy::manual_map,
    clippy::ref_option
)]

pub mod media;
pub mod policy;
pub mod post;
pub mod slug;

pub use media::media_key_ok;
pub use policy::{PublicPost, filter_public, reveal};
pub use post::{Post, State, WORDS_PER_MINUTE, reading_minutes};
pub use slug::{SLUG_MAX, make_slug};
