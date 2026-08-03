//! Shared foundation for DraftOS tools.
//!
//! `draftos-common` holds the small pieces every DraftOS binary needs: which
//! [`Edition`] it is running on (which decides the system-management backend),
//! and the project's [`brand`] identity constants. It is deliberately
//! dependency-free (std only) so it stays cheap to link into both CLI tools and
//! libcosmic GUI apps.

pub mod brand;
pub mod edition;

pub use edition::Edition;
