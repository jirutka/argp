// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>

//! This module provides a lightweight, zero-dependencies implementation of a
//! function to get the terminal width on Unix systems. It's replaced with a
//! no-op implementation on non-unix systems or when the `term_size` feature
//! is disabled.

#[cfg(all(unix, feature = "term_size"))]
mod unix;

#[cfg(all(unix, feature = "term_size"))]
pub use unix::term_cols;

#[cfg(not(all(unix, feature = "term_size")))]
/// This is a no-op implementation for non-unix systems that always returns
/// `None`.
pub fn term_cols() -> Option<usize> {
    None
}
