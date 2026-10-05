//! The editor preview (spec 6.7): `logbook_render::render` compiled to WebAssembly,
//! so the preview and the published post use the same code.
//!
//! The JS side writes UTF-8 markdown into a buffer from [`abi::buf_alloc`], calls
//! [`abi::render`], reads the HTML, and frees both buffers with [`abi::buf_free`].
//! No bindings generator: the interface is three plain functions.

/// Renders markdown to sanitized HTML. The safe core of the shim.
#[must_use]
pub fn render_bytes(input: &[u8]) -> Vec<u8> {
    logbook_render::render(&String::from_utf8_lossy(input)).into_bytes()
}

/// The memory interface with JavaScript. The only unsafe code in the project.
#[allow(unsafe_code)]
pub mod abi {
    /// Allocates `len` bytes in WASM memory for the caller to fill.
    #[unsafe(no_mangle)]
    pub extern "C" fn buf_alloc(len: usize) -> *mut u8 {
        let mut v = Vec::<u8>::with_capacity(len);
        let p = v.as_mut_ptr();
        std::mem::forget(v);
        p
    }

    /// Frees a buffer from [`buf_alloc`] or [`render`].
    ///
    /// # Safety
    ///
    /// `ptr` and `len` must come from one earlier `buf_alloc(len)` or `render` result,
    /// and must not be freed twice.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn buf_free(ptr: *mut u8, len: usize) {
        // SAFETY: the caller passes a pointer and capacity from `buf_alloc` or `render`.
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
    }

    /// Renders `len` bytes of markdown at `ptr`. Returns `(html_ptr << 32) | html_len`.
    /// The caller frees the input and the output with [`buf_free`].
    ///
    /// # Safety
    ///
    /// `ptr` must point to `len` initialized bytes from [`buf_alloc`].
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn render(ptr: *const u8, len: usize) -> u64 {
        // SAFETY: the caller wrote `len` bytes at `ptr`, in a buffer from `buf_alloc`.
        let input = unsafe { std::slice::from_raw_parts(ptr, len) };
        let mut html = super::render_bytes(input);
        html.shrink_to_fit();
        let (p, n) = (html.as_mut_ptr(), html.len());
        std::mem::forget(html);
        ((p as u64) << 32) | n as u64
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_like_the_server() {
        let md = "# Hi\n\n```rust\nfn x() {}\n```\n\n<script>x</script>";
        assert_eq!(
            super::render_bytes(md.as_bytes()),
            logbook_render::render(md).into_bytes()
        );
    }
}
