//! Spec 7.3 `paint_ops`: any sequence of Paint actions on a small picture. No panic.
//! Both canvases stay valid after every action. Undo, as many times as there were
//! actions, gives back the first picture.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_paint::valid;
use logbook_paint_wasm::{Paint, UNDO_LIMIT, WHITE, ops};

/// Reads small numbers from the input. Zero when the input runs out.
struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn u8(&mut self) -> u8 {
        let Some((&b, rest)) = self.0.split_first() else { return 0 };
        self.0 = rest;
        b
    }
    /// A coordinate from -8 to 39: a little outside the picture on each side.
    fn coord(&mut self) -> i32 {
        i32::from(self.u8() % 48) - 8
    }
    fn color(&mut self) -> u32 {
        [WHITE, 0xFF00_0000, 0xFF00_00FF, 0][usize::from(self.u8() % 4)]
    }
}

fuzz_target!(|data: &[u8]| {
    let mut b = Bytes(data);
    let mut p = Paint::new();
    ops::blank(&mut p, 0, i32::from(b.u8() % 32) + 1, i32::from(b.u8() % 32) + 1, WHITE);
    let first = p.canvases[0].clone();
    let mut n = 0;
    while !b.0.is_empty() && n < UNDO_LIMIT {
        p.snapshot();
        n += 1;
        let (x, y, x2, y2) = (b.coord(), b.coord(), b.coord(), b.coord());
        match b.u8() % 16 {
            0 => ops::line(&mut p, x, y, x2, y2, i32::from(b.u8() % 8), b.color()),
            1 => ops::flood_fill(&mut p, x, y, b.color()),
            2 => ops::fill_rect(&mut p, x, y, x2, y2, b.color()),
            3 => ops::rect_outline(&mut p, x, y, x2, y2, i32::from(b.u8() % 6), b.color()),
            4 => ops::color_replace(&mut p, x, y, x2, y2, b.color(), b.color()),
            5 => ops::flip_h(&mut p, u32::from(b.u8() % 2)),
            6 => ops::flip_v(&mut p, u32::from(b.u8() % 2)),
            7 => ops::rotate90(&mut p, u32::from(b.u8() % 2)),
            8 => ops::invert(&mut p, u32::from(b.u8() % 2)),
            9 => ops::black_white(&mut p, u32::from(b.u8() % 2)),
            10 => ops::stretch(&mut p, u32::from(b.u8() % 2), x2, y2),
            11 => ops::skew_x(&mut p, u32::from(b.u8() % 2), x * 100, b.color()),
            12 => ops::skew_y(&mut p, u32::from(b.u8() % 2), y * 100, b.color()),
            13 => ops::resize(&mut p, x2, y2, b.color()),
            14 => ops::sel_copy(&mut p, x, y, x2, y2),
            _ => ops::sel_blit(&mut p, x, y, b.u8() % 2 == 1, b.color()),
        }
        assert!(valid(&p.canvases[0]) && valid(&p.canvases[1]));
    }
    for _ in 0..n {
        assert!(p.undo());
    }
    assert_eq!(p.canvases[0], first);
});
