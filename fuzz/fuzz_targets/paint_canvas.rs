//! Spec 7.3 `paint_canvas`: any canvas, also one whose fields do not agree, and any
//! numbers. No pixel function panics (P10). A function does not change an invalid canvas.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_paint as lp;
use logbook_paint::Canvas;

fuzz_target!(|input: (u8, u8, i32, i32, i32, i32, u32, Vec<u32>)| {
    let (w, h, x, y, a, b, col, px) = input;
    let c = Canvas { w: usize::from(w), h: usize::from(h), px };
    let mut m = c.clone();
    lp::set(&mut m, x, y, col);
    let _ = lp::get(&m, x, y);
    lp::fill_rect(&mut m, x, y, a, b, col);
    lp::rect_outline(&mut m, x, y, a, b, a, col);
    lp::line(&mut m, x, y, x.wrapping_add(a % 512), y.wrapping_add(b % 512), b, col);
    lp::flood_fill(&mut m, x, y, col);
    lp::color_replace(&mut m, x, y, a, b, col, !col);
    lp::flip_h(&mut m);
    lp::flip_v(&mut m);
    lp::invert(&mut m);
    lp::to_black_white(&mut m);
    let r = lp::rotate90(&m);
    let s = lp::copy_region(&m, x, y, a % 64, b % 64);
    lp::blit(&mut m, &s, y, x, a % 2 == 0, col);
    let _ = lp::resize(&r, usize::from(w), usize::from(h), col);
    let _ = lp::stretch(&m, usize::from(h), usize::from(w));
    let _ = lp::skew_x(&m, a, col);
    let _ = lp::skew_y(&m, b, col);
    let k = lp::clip_rect(x, y, a, b, usize::from(w), usize::from(h));
    assert!(k.x + k.w <= usize::from(w) && k.y + k.h <= usize::from(h));
    if !lp::valid(&c) {
        assert_eq!(m, c);
    }
});
