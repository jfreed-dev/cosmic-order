// SPDX-License-Identifier: GPL-3.0-only

//! Tool config generators
//!
//! Each sub-module generates configuration files for a specific tool
//! from the extracted [`ColorPalette`](crate::colors::ColorPalette).

pub mod btop;
pub mod fzf;
pub mod ghostty;
pub mod lazygit;
pub mod nvim;
pub mod zellij;
