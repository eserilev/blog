//! The markdown pipeline: comrak, then syntect, then ammonia (spec 6.7).
//!
//! The server uses it at save time. The editor preview uses the same code,
//! compiled to WebAssembly. Step 2 of the build adds `render()`.
