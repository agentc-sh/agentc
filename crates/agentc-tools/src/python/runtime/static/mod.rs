// SPDX-FileCopyrightText: 2026 Timothy Pogue
//
// SPDX-License-Identifier: LicenseRef-Proprietary

mod context;
mod embed;
mod errors;
mod interpreter;
mod runtime;

pub use embed::EmbeddedTree;
pub use runtime::StaticRuntime;

#[doc(hidden)]
pub use include_dir as __include_dir;

#[macro_export]
macro_rules! embed_dir {
    ($path:expr) => {{
        use $crate::python::runtime::r#static::__include_dir as include_dir;

        $crate::python::runtime::r#static::EmbeddedTree::from(include_dir::include_dir!($path))
    }};
}

pub use crate::embed_dir;
