//! # crustf-jar-builder
//!
//! Pure-Rust, dependency-free writer for JAR archives (and the ZIP subset
//! they sit on). The crate is `no_std` when built without the `std`
//! feature, relies only on `alloc`, and has no transitive OS or IO
//! dependencies — so it is safe to compile for `wasm32-unknown-unknown`
//! and similar targets that carry nothing but an allocator.
//!
//! ```no_run
//! use crustf_jar_builder::{JarBuilder, Manifest};
//!
//! let class_bytes: Vec<u8> = /* from crustf */ Vec::new();
//! let jar = JarBuilder::new()
//!     .main_class("HelloWorld")
//!     .file("HelloWorld.class", class_bytes)
//!     .build()
//!     .unwrap();
//! # let _ = jar;
//! # let _ = Manifest::new();
//! ```
//!
//! The archive writer only supports the `STORED` (uncompressed) method.
//! That keeps the crate dependency-free and reproducible byte-for-byte,
//! which is enough for `java -jar` and every other consumer of the JAR
//! format.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod builder;
pub mod crc32;
pub mod error;
pub mod manifest;
pub mod zip;

pub use builder::{JarBuilder, MANIFEST_PATH};
pub use crc32::{crc32, Hasher as Crc32Hasher};
pub use error::{Error, Result};
pub use manifest::Manifest;
pub use zip::ZipWriter;
