//! Trending analytics integration with `formulae.brew.sh` (always on)
//! and historical trending data from `brew-browser.zerologic.com`
//! (opt-in, v0.4.0+).

pub mod cache;
pub mod client;
pub mod history;
pub mod velocity;
