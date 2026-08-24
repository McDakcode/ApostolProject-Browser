// Made by MrDuck && Ox-Alpha
//! Command handlers grouped by domain.
//!
//! Each submodule owns one feature domain and mirrors the frontend module
//! of the same name under `ui/js/` (see `ARCHITECTURE.md`):
//!
//! | Rust            | Frontend                    |
//! |-----------------|-----------------------------|
//! | profiles        | js/panels/04-…              |
//! | bookmarks       | js/panels/08-…              |
//! | history         | js/panels/08-…              |
//! | notes           | js/notes/09-… + graph/11-…  |
//! | session         | js/tabs/06-…                |
//! | workspaces      | js/tabs/06-…                |
//! | downloads       | js/shell/* + toasts         |
//! | pages           | js/tabs/06-… (tab engine)   |
//! | palette         | js/panels/08-… (Ctrl+K)     |
//! | privacy/network | js/panels/08-…              |
//! | vault           | js/panels/08-…              |
//! | ai              | js/panels/08-…              |
//! | exts            | js/panels/08-…              |

pub mod ai;
pub mod bookmarks;
pub mod debug;
pub mod downloads;
pub mod exts;
pub mod history;
pub mod network;
pub mod notes;
pub mod pages;
pub mod palette;
pub mod privacy;
pub mod profiles;
pub mod session;
pub mod vault;
pub mod workspaces;

pub(crate) use ai::*;
pub(crate) use bookmarks::*;
pub(crate) use debug::*;
pub(crate) use downloads::*;
pub(crate) use exts::*;
pub(crate) use history::*;
pub(crate) use network::*;
pub(crate) use notes::*;
pub(crate) use pages::*;
pub(crate) use palette::*;
pub(crate) use privacy::*;
pub(crate) use profiles::*;
pub(crate) use session::*;
pub(crate) use vault::*;
pub(crate) use workspaces::*;

// Made by MrDuck && Ox-Alpha