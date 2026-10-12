//! Paint (spec 11.9): `logbook-paint` compiled to WebAssembly.
//!
//! The picture lives in WASM memory. JS draws it through a `Uint8ClampedArray` view
//! on the pixel buffer: [`abi::paint_ptr`] gives the address, and the bytes are
//! RGBA, the same as `ImageData`. JS makes a new view after every call, because a
//! call can move the buffer or grow the memory.
//!
//! There are two canvases. Target 0 is the picture. Target 1 is the floating
//! selection. Undo keeps whole copies of the picture: at most [`UNDO_LIMIT`] steps,
//! and at most [`UNDO_BYTES`] in total. WASM memory never shrinks, so the byte cap
//! sets the most memory that Paint can use for undo.
//!
//! No bindings generator: the interface is plain `extern "C"` functions of integers.

// The pixel functions take `x, y, w, h` like the canvas API does.
#![allow(clippy::many_single_char_names)]

use logbook_paint as lp;
use logbook_paint::Canvas;

/// The most undo steps that Paint keeps.
pub const UNDO_LIMIT: usize = 30;
/// The most bytes that the undo and repeat steps hold together: 128 MiB. A small
/// picture keeps all 30 steps. A 4096 × 4096 picture (64 MiB) keeps 2.
pub const UNDO_BYTES: usize = 128 << 20;
/// Opaque white.
pub const WHITE: u32 = 0xFFFF_FFFF;

/// The state behind the exported functions.
#[derive(Debug, Clone)]
pub struct Paint {
    /// Target 0: the picture. Target 1: the floating selection.
    pub canvases: [Canvas; 2],
    undo: Vec<Canvas>,
    redo: Vec<Canvas>,
}

impl Default for Paint {
    fn default() -> Self {
        Self::new()
    }
}

impl Paint {
    /// A white 560 × 340 picture with no history.
    #[must_use]
    pub fn new() -> Self {
        Paint {
            canvases: [
                lp::new_canvas(560, 340, WHITE),
                lp::new_canvas(1, 1, lp::TRANSPARENT),
            ],
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The canvas for target `t`. Any `t` other than 1 is the picture.
    pub fn target(&mut self, t: u32) -> &mut Canvas {
        &mut self.canvases[usize::from(t == 1)]
    }

    /// The picture.
    pub fn main(&mut self) -> &mut Canvas {
        &mut self.canvases[0]
    }

    /// Saves the picture before a change. A new change clears Repeat. The oldest
    /// steps go first when the history passes [`UNDO_LIMIT`] or [`UNDO_BYTES`]. The
    /// newest step always stays.
    pub fn snapshot(&mut self) {
        self.redo.clear();
        // Make room first, so the new copy can reuse the memory of the oldest one.
        let incoming = self.canvases[0].px.len() * 4;
        while !self.undo.is_empty()
            && (self.undo.len() >= UNDO_LIMIT || self.history_bytes() + incoming > UNDO_BYTES)
        {
            self.undo.remove(0);
        }
        self.undo.push(self.canvases[0].clone());
    }

    /// The bytes of pixels that the undo and repeat steps hold.
    #[must_use]
    pub fn history_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(&self.redo)
            .map(|c| c.px.len() * 4)
            .sum()
    }

    /// Goes back one step. Returns false if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        let cur = std::mem::replace(&mut self.canvases[0], prev);
        self.redo.push(cur);
        true
    }

    /// Goes forward one undone step. Returns false if there is nothing to repeat.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let cur = std::mem::replace(&mut self.canvases[0], next);
        self.undo.push(cur);
        true
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Forgets all undo and repeat steps (File ▸ New, File ▸ Open).
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

/// A size from JS: negative becomes 0, then `clamp_dim` makes it `1..=MAX_DIM`.
fn dim(v: i32) -> usize {
    lp::clamp_dim(usize::try_from(v).unwrap_or(0))
}

/// The operations, as plain functions on [`Paint`], so the tests call the same code
/// as the exports.
pub mod ops {
    use super::{Paint, dim, lp};

    pub fn line(p: &mut Paint, x0: i32, y0: i32, x1: i32, y1: i32, width: i32, color: u32) {
        lp::line(p.main(), x0, y0, x1, y1, width, color);
    }
    pub fn fill_rect(p: &mut Paint, x: i32, y: i32, w: i32, h: i32, color: u32) {
        lp::fill_rect(p.main(), x, y, w, h, color);
    }
    pub fn rect_outline(p: &mut Paint, x: i32, y: i32, w: i32, h: i32, lw: i32, color: u32) {
        lp::rect_outline(p.main(), x, y, w, h, lw, color);
    }
    pub fn flood_fill(p: &mut Paint, x: i32, y: i32, color: u32) {
        lp::flood_fill(p.main(), x, y, color);
    }
    pub fn color_replace(p: &mut Paint, x: i32, y: i32, w: i32, h: i32, from: u32, to: u32) {
        lp::color_replace(p.main(), x, y, w, h, from, to);
    }
    pub fn flip_h(p: &mut Paint, t: u32) {
        lp::flip_h(p.target(t));
    }
    pub fn flip_v(p: &mut Paint, t: u32) {
        lp::flip_v(p.target(t));
    }
    pub fn rotate90(p: &mut Paint, t: u32) {
        let c = p.target(t);
        *c = lp::rotate90(c);
    }
    pub fn invert(p: &mut Paint, t: u32) {
        lp::invert(p.target(t));
    }
    pub fn black_white(p: &mut Paint, t: u32) {
        lp::to_black_white(p.target(t));
    }
    pub fn stretch(p: &mut Paint, t: u32, w: i32, h: i32) {
        let c = p.target(t);
        *c = lp::stretch(c, dim(w), dim(h));
    }
    pub fn skew_x(p: &mut Paint, t: u32, num: i32, bg: u32) {
        let c = p.target(t);
        *c = lp::skew_x(c, num, bg);
    }
    pub fn skew_y(p: &mut Paint, t: u32, num: i32, bg: u32) {
        let c = p.target(t);
        *c = lp::skew_y(c, num, bg);
    }
    /// A new blank canvas for target `t`, for JS to fill (File ▸ Open, shapes, text).
    pub fn blank(p: &mut Paint, t: u32, w: i32, h: i32, color: u32) {
        *p.target(t) = lp::new_canvas(dim(w), dim(h), color);
    }
    pub fn resize(p: &mut Paint, w: i32, h: i32, bg: u32) {
        let c = p.main();
        *c = lp::resize(c, dim(w), dim(h), bg);
    }
    /// Copies part of the picture into the selection.
    pub fn sel_copy(p: &mut Paint, x: i32, y: i32, w: i32, h: i32) {
        p.canvases[1] = lp::copy_region(&p.canvases[0], x, y, w, h);
    }
    /// Draws the selection onto the picture.
    pub fn sel_blit(p: &mut Paint, x: i32, y: i32, use_key: bool, key: u32) {
        let [main, sel] = &mut p.canvases;
        lp::blit(main, sel, x, y, use_key, key);
    }
}

/// The exported functions. The only code here that is not safe Rust is the
/// `#[unsafe(no_mangle)]` attribute itself: no function reads or writes raw memory.
#[allow(unsafe_code)]
pub mod abi {
    use super::{Paint, ops};
    use std::cell::RefCell;

    thread_local! {
        static PAINT: RefCell<Paint> = RefCell::new(Paint::new());
    }

    fn with<R>(f: impl FnOnce(&mut Paint) -> R) -> R {
        PAINT.with(|p| f(&mut p.borrow_mut()))
    }

    fn u(v: usize) -> u32 {
        u32::try_from(v).unwrap_or(u32::MAX)
    }

    /// A new white picture of `w × h`, with no history.
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_init(w: i32, h: i32, color: u32) {
        with(|p| {
            *p = Paint::new();
            ops::blank(p, 0, w, h, color);
        });
    }
    /// The address of the pixels of target `t`. JS reads `4 × w × h` bytes there.
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_ptr(t: u32) -> *mut u32 {
        with(|p| p.target(t).px.as_mut_ptr())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_width(t: u32) -> u32 {
        with(|p| u(p.target(t).w))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_height(t: u32) -> u32 {
        with(|p| u(p.target(t).h))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_blank(t: u32, w: i32, h: i32, color: u32) {
        with(|p| ops::blank(p, t, w, h, color));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_snapshot() {
        with(Paint::snapshot);
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_undo() -> u32 {
        with(|p| u32::from(p.undo()))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_redo() -> u32 {
        with(|p| u32::from(p.redo()))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_can_undo() -> u32 {
        with(|p| u32::from(p.can_undo()))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_can_redo() -> u32 {
        with(|p| u32::from(p.can_redo()))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_clear_history() {
        with(Paint::clear_history);
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_get(x: i32, y: i32) -> u32 {
        with(|p| logbook_paint::get(p.main(), x, y))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_line(x0: i32, y0: i32, x1: i32, y1: i32, width: i32, color: u32) {
        with(|p| ops::line(p, x0, y0, x1, y1, width, color));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_fill_rect(x: i32, y: i32, w: i32, h: i32, color: u32) {
        with(|p| ops::fill_rect(p, x, y, w, h, color));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_rect_outline(x: i32, y: i32, w: i32, h: i32, lw: i32, color: u32) {
        with(|p| ops::rect_outline(p, x, y, w, h, lw, color));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_flood_fill(x: i32, y: i32, color: u32) {
        with(|p| ops::flood_fill(p, x, y, color));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_color_replace(x: i32, y: i32, w: i32, h: i32, from: u32, to: u32) {
        with(|p| ops::color_replace(p, x, y, w, h, from, to));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_flip_h(t: u32) {
        with(|p| ops::flip_h(p, t));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_flip_v(t: u32) {
        with(|p| ops::flip_v(p, t));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_rotate90(t: u32) {
        with(|p| ops::rotate90(p, t));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_invert(t: u32) {
        with(|p| ops::invert(p, t));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_black_white(t: u32) {
        with(|p| ops::black_white(p, t));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_stretch(t: u32, w: i32, h: i32) {
        with(|p| ops::stretch(p, t, w, h));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_skew_x(t: u32, num: i32, bg: u32) {
        with(|p| ops::skew_x(p, t, num, bg));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_skew_y(t: u32, num: i32, bg: u32) {
        with(|p| ops::skew_y(p, t, num, bg));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_resize(w: i32, h: i32, bg: u32) {
        with(|p| ops::resize(p, w, h, bg));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_sel_copy(x: i32, y: i32, w: i32, h: i32) {
        with(|p| ops::sel_copy(p, x, y, w, h));
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn paint_sel_blit(x: i32, y: i32, use_key: u32, key: u32) {
        with(|p| ops::sel_blit(p, x, y, use_key != 0, key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// One user action. Each one takes a snapshot first, the same as the JS does.
    #[derive(Debug, Clone)]
    enum Op {
        Line(i32, i32, i32, i32, i32, u32),
        Fill(i32, i32, u32),
        Rect(i32, i32, i32, i32, u32),
        Erase(i32, i32, u32, u32),
        FlipH,
        FlipV,
        Rotate,
        Invert,
        Resize(i32, i32),
        Stretch(i32, i32),
        Skew(i32),
        Paste(i32, i32, i32, i32, i32, i32),
    }

    fn op() -> impl Strategy<Value = Op> {
        let c = -4i32..24;
        let col = prop_oneof![Just(WHITE), Just(0xFF00_0000u32), Just(0xFF00_00FFu32)];
        prop_oneof![
            (
                c.clone(),
                c.clone(),
                c.clone(),
                c.clone(),
                1i32..4,
                col.clone()
            )
                .prop_map(|(a, b, x, y, w, k)| Op::Line(a, b, x, y, w, k)),
            (c.clone(), c.clone(), col.clone()).prop_map(|(x, y, k)| Op::Fill(x, y, k)),
            (c.clone(), c.clone(), c.clone(), c.clone(), col.clone())
                .prop_map(|(x, y, w, h, k)| Op::Rect(x, y, w, h, k)),
            (c.clone(), c.clone(), col.clone(), col).prop_map(|(x, y, a, b)| Op::Erase(x, y, a, b)),
            Just(Op::FlipH),
            Just(Op::FlipV),
            Just(Op::Rotate),
            Just(Op::Invert),
            (1i32..30, 1i32..30).prop_map(|(w, h)| Op::Resize(w, h)),
            (1i32..30, 1i32..30).prop_map(|(w, h)| Op::Stretch(w, h)),
            (-2000i32..2000).prop_map(Op::Skew),
            (c.clone(), c.clone(), 1i32..10, 1i32..10, c.clone(), c)
                .prop_map(|(a, b, w, h, x, y)| Op::Paste(a, b, w, h, x, y)),
        ]
    }

    fn apply(p: &mut Paint, o: &Op) {
        p.snapshot();
        match *o {
            Op::Line(a, b, x, y, w, k) => ops::line(p, a, b, x, y, w, k),
            Op::Fill(x, y, k) => ops::flood_fill(p, x, y, k),
            Op::Rect(x, y, w, h, k) => ops::rect_outline(p, x, y, w, h, 1, k),
            Op::Erase(x, y, a, b) => ops::color_replace(p, x, y, 4, 4, a, b),
            Op::FlipH => ops::flip_h(p, 0),
            Op::FlipV => ops::flip_v(p, 0),
            Op::Rotate => ops::rotate90(p, 0),
            Op::Invert => ops::invert(p, 0),
            Op::Resize(w, h) => ops::resize(p, w, h, WHITE),
            Op::Stretch(w, h) => ops::stretch(p, 0, w, h),
            Op::Skew(n) => ops::skew_x(p, 0, n, WHITE),
            Op::Paste(a, b, w, h, x, y) => {
                ops::sel_copy(p, a, b, w, h);
                ops::rotate90(p, 1);
                ops::sel_blit(p, x, y, false, 0);
            }
        }
    }

    fn small() -> Paint {
        let mut p = Paint::new();
        ops::blank(&mut p, 0, 16, 12, WHITE);
        p
    }

    #[test]
    fn undo_on_a_new_picture_does_nothing() {
        let mut p = small();
        assert!(!p.undo());
        assert!(!p.redo());
    }

    #[test]
    fn a_big_picture_keeps_fewer_steps() {
        let mut p = Paint::new();
        ops::blank(&mut p, 0, 4096, 4096, WHITE);
        for _ in 0..10 {
            p.snapshot();
            assert!(p.history_bytes() <= UNDO_BYTES);
        }
        let mut n = 0;
        while p.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_BYTES / (4096 * 4096 * 4));
    }

    #[test]
    fn a_picture_over_the_cap_still_keeps_one_step() {
        let mut p = Paint::new();
        p.canvases[0] = logbook_paint::Canvas {
            w: 1,
            h: 1,
            px: vec![0; UNDO_BYTES / 4 + 1],
        };
        p.snapshot();
        p.snapshot();
        assert!(p.undo());
        assert!(!p.undo());
    }

    #[test]
    fn the_history_keeps_at_most_the_limit() {
        let mut p = small();
        for i in 0..(UNDO_LIMIT + 10) {
            p.snapshot();
            ops::fill_rect(&mut p, 0, 0, 1, 1, u32::try_from(i).unwrap());
        }
        let mut n = 0;
        while p.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_LIMIT);
    }

    proptest! {
        /// Any sequence of actions, then the same number of undos, gives the first picture.
        #[test]
        fn undo_restores_the_first_picture(ops_ in prop::collection::vec(op(), 0..UNDO_LIMIT)) {
            let mut p = small();
            let first = p.canvases[0].clone();
            for o in &ops_ {
                apply(&mut p, o);
            }
            for _ in &ops_ {
                prop_assert!(p.undo());
            }
            prop_assert_eq!(&p.canvases[0], &first);
        }

        /// Undo all, then repeat all, gives the last picture.
        #[test]
        fn redo_restores_the_last_picture(ops_ in prop::collection::vec(op(), 0..12)) {
            let mut p = small();
            for o in &ops_ {
                apply(&mut p, o);
            }
            let last = p.canvases[0].clone();
            while p.undo() {}
            while p.redo() {}
            prop_assert_eq!(&p.canvases[0], &last);
        }

        /// After any actions, both canvases are valid, so JS can always read `w × h` pixels.
        #[test]
        fn the_canvases_stay_valid(ops_ in prop::collection::vec(op(), 0..40)) {
            let mut p = small();
            for o in &ops_ {
                apply(&mut p, o);
                prop_assert!(logbook_paint::valid(&p.canvases[0]));
                prop_assert!(logbook_paint::valid(&p.canvases[1]));
            }
        }
    }
}
