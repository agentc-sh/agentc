// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

mod file;
mod filesystem;
mod types;

#[doc(hidden)]
pub use include_dir::include_dir as __include_dir;

pub use file::EmbeddedFile;
pub use filesystem::EmbeddedFs;
pub use types::{EmbeddedDirectory, EmbeddedSource};

#[macro_export]
macro_rules! embedded_file {
    ($($path:tt)+) => {
        $crate::embedded::EmbeddedFs::file(include_bytes!($($path)+))
    };
}

#[macro_export]
macro_rules! embedded_dir {
    ($($path:tt)+) => {
        $crate::embedded::EmbeddedFs::directory(
            $crate::embedded::EmbeddedDirectory::__new(
                $crate::embedded::__include_dir!($($path)+),
            ),
        )
    };
}
