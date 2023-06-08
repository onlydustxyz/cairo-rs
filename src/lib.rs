//! An implementation of the Cairo virtual machine
//!
//! # Feature Flags
//! - `skip_next_instruction_hint`: Enable the `skip_next_instruction()` hint. Not enabled by default.
//! - `hooks`: Enable [Hooks](vm::hooks) support for the [VirtualMachine](vm::vm_core::VirtualMachine). Not enabled by default.
//! - `with_mimalloc`: Use [MiMalloc](https://crates.io/crates/mimalloc) as the program global allocator.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(warnings)]
#![forbid(unsafe_code)]
#![cfg_attr(any(target_arch = "wasm32", not(feature = "std")), no_std)]

#[cfg(feature = "std")]
include!("./with_std.rs");
#[cfg(not(feature = "std"))]
include!("./without_std.rs");

mod stdlib {
    pub mod collections {
        #[cfg(feature = "std")]
        pub use crate::with_std::collections::*;
        #[cfg(not(feature = "std"))]
        pub use crate::without_std::collections::*;
    }

    pub mod borrow {
        #[cfg(feature = "std")]
        pub use crate::with_std::borrow::*;
        #[cfg(not(feature = "std"))]
        pub use crate::without_std::borrow::*;
    }

    pub mod prelude {
        pub use crate::stdlib::{
            borrow::ToOwned,
            boxed::Box,
            clone::Clone,
            cmp::{Eq, PartialEq, Reverse},
            fmt,
            iter::IntoIterator,
            str::FromStr,
            string::{String, ToString},
            vec::Vec,
        };
    }

    #[cfg(feature = "std")]
    pub use crate::with_std::*;
    #[cfg(not(feature = "std"))]
    pub use crate::without_std::*;
}

pub extern crate felt;
pub mod cairo_run;
pub mod hint_processor;
pub mod math_utils;
pub mod serde;
pub mod types;
pub mod utils;
pub mod vm;

#[cfg(test)]
mod tests;
