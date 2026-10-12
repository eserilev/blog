//! Unit tests and property tests. The proptests check the same rules as the Lean
//! theorems P1–P10, on small canvases, so a broken change fails here first.

use super::*;
use proptest::prelude::*;
use std::collections::VecDeque;

const BLACK: u32 = 0xFF00_0000;
const WHITE: u32 = 0xFFFF_FFFF;
const RED: u32 = 0xFF00_00FF;
const BLUE: u32 = 0xFFFF_0000;

fn color() -> impl Strategy<Value = u32> {
    prop_oneof![Just(BLACK), Just(WHITE), Just(RED), Just(TRANSPARENT)]
}

/// A small valid canvas with few colors, so fills meet real borders.
fn canvas() -> impl Strategy<Value = Canvas> {
    (1usize..12, 1usize..12).prop_flat_map(|(w, h)| {
        prop::collection::vec(color(), w * h).prop_map(move |px| Canvas { w, h, px })
    })
}

/// Any canvas, also one whose fields do not agree.
fn any_canvas() -> impl Strategy<Value = Canvas> {
    prop_oneof![
        canvas(),
        (0usize..6, 0usize..6, prop::collection::vec(color(), 0..40))
            .prop_map(|(w, h, px)| Canvas { w, h, px }),
        (
            prop_oneof![Just(0usize), Just(MAX_DIM + 1), Just(usize::MAX)],
            0usize..3
        )
            .prop_map(|(w, h)| Canvas {
                w,
                h,
                px: vec![BLACK; h]
            }),
    ]
}

fn coord() -> impl Strategy<Value = i32> {
    prop_oneof![-3i32..15, Just(i32::MIN), Just(i32::MAX), any::<i32>()]
}

/// Reference flood fill: a breadth-first search with a visited set.
fn reference_fill(c: &Canvas, x: i32, y: i32, color: u32) -> Canvas {
    let mut out = c.clone();
    let Some(start) = index(c, x, y) else {
        return out;
    };
    let target = c.px[start];
    if target == color {
        return out;
    }
    let mut seen = vec![false; c.px.len()];
    let mut q = VecDeque::from([start]);
    seen[start] = true;
    while let Some(i) = q.pop_front() {
        out.px[i] = color;
        let (cx, cy) = (i % c.w, i / c.w);
        let mut near = Vec::new();
        if cx > 0 {
            near.push(i - 1);
        }
        if cx + 1 < c.w {
            near.push(i + 1);
        }
        if cy > 0 {
            near.push(i - c.w);
        }
        if cy + 1 < c.h {
            near.push(i + c.w);
        }
        for j in near {
            if !seen[j] && c.px[j] == target {
                seen[j] = true;
                q.push_back(j);
            }
        }
    }
    out
}

#[test]
fn new_canvas_clamps_the_sides() {
    let c = new_canvas(0, MAX_DIM + 5, WHITE);
    assert_eq!((c.w, c.h), (1, MAX_DIM));
    assert!(valid(&c));
}

#[test]
fn line_examples() {
    assert_eq!(
        line_points(0, 0, 3, 1),
        vec![(0, 0), (1, 0), (2, 1), (3, 1)]
    );
    assert_eq!(line_points(2, 2, 2, 2), vec![(2, 2)]);
    assert_eq!(
        line_points(0, 3, 0, 0),
        vec![(0, 3), (0, 2), (0, 1), (0, 0)]
    );
}

#[test]
fn a_line_across_the_whole_range() {
    let pts = line_points(i32::MIN, 5, i32::MAX, -9);
    assert_eq!(pts.len(), 2 * COORD_LIMIT as usize + 1);
    assert_eq!(pts[0], (-COORD_LIMIT, 5));
    assert_eq!(*pts.last().unwrap(), (COORD_LIMIT, -9));
}

#[test]
fn flood_fill_stops_at_a_wall() {
    let mut c = new_canvas(5, 1, WHITE);
    set(&mut c, 2, 0, BLACK);
    flood_fill(&mut c, 0, 0, RED);
    assert_eq!(c.px, vec![RED, RED, BLACK, WHITE, WHITE]);
}

#[test]
fn flood_fill_on_the_largest_canvas_ends() {
    let mut c = new_canvas(MAX_DIM, 64, WHITE);
    flood_fill(&mut c, 7, 7, BLUE);
    assert!(c.px.iter().all(|&p| p == BLUE));
}

#[test]
fn rotate_moves_the_corner() {
    let mut c = new_canvas(3, 2, WHITE);
    set(&mut c, 0, 0, RED);
    let r = rotate90(&c);
    assert_eq!((r.w, r.h), (2, 3));
    assert_eq!(get(&r, 1, 0), RED);
}

#[test]
fn skew_grows_the_canvas() {
    let c = new_canvas(4, 5, RED);
    assert_eq!(skew_x(&c, 1000, WHITE).w, 8);
    assert_eq!(skew_x(&c, -1000, WHITE).w, 8);
    assert_eq!(skew_y(&c, 500, WHITE).h, 6);
}

proptest! {
    /// P1: the fill changes exactly the connected area of the start color.
    #[test]
    fn flood_fill_matches_the_reference(c in canvas(), x in -2i32..14, y in -2i32..14, col in color()) {
        let mut got = c.clone();
        flood_fill(&mut got, x, y, col);
        prop_assert_eq!(got, reference_fill(&c, x, y, col));
    }

    /// P3: the line starts and ends at the given points, has no gaps, and stays in
    /// the bounding box.
    #[test]
    fn line_points_follow_the_rules(x0 in coord(), y0 in coord(), dx in -300i32..300, dy in -300i32..300) {
        // Short lines anywhere. `a_line_across_the_whole_range` checks a long one.
        let (x1, y1) = (x0.saturating_add(dx), y0.saturating_add(dy));
        let (a, b, c, d) = (clamp_coord(x0), clamp_coord(y0), clamp_coord(x1), clamp_coord(y1));
        let pts = line_points(x0, y0, x1, y1);
        prop_assert_eq!(pts[0], (a, b));
        prop_assert_eq!(*pts.last().unwrap(), (c, d));
        let n = (a - c).abs().max((b - d).abs()) as usize;
        prop_assert_eq!(pts.len(), n + 1);
        for w in pts.windows(2) {
            prop_assert!((w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1);
        }
        for &(x, y) in &pts {
            prop_assert!(x >= a.min(c) && x <= a.max(c) && y >= b.min(d) && y <= b.max(d));
        }
    }

    /// A 1-pixel line changes only the pixels of `line_points`.
    #[test]
    fn a_thin_line_sets_only_its_points(c in canvas(), x0 in -3i32..15, y0 in -3i32..15, x1 in -3i32..15, y1 in -3i32..15) {
        let mut got = c.clone();
        line(&mut got, x0, y0, x1, y1, 1, BLUE);
        let pts = line_points(x0, y0, x1, y1);
        for y in 0..c.h as i32 {
            for x in 0..c.w as i32 {
                let on = pts.contains(&(x, y));
                prop_assert_eq!(get(&got, x, y), if on { BLUE } else { get(&c, x, y) });
            }
        }
    }

    /// P4: the outline changes only pixels within `lw` of an edge of the rectangle.
    #[test]
    fn rect_outline_touches_only_the_border(c in canvas(), x in -3i32..12, y in -3i32..12, w in -2i32..14, h in -2i32..14, lw in 1i32..5) {
        let mut got = c.clone();
        rect_outline(&mut got, x, y, w, h, lw, BLUE);
        for py in 0..c.h as i32 {
            for px in 0..c.w as i32 {
                let inside = w > 0 && h > 0 && px >= x && px < x + w && py >= y && py < y + h;
                let border = inside && (px < x + lw || px >= x + w - lw || py < y + lw || py >= y + h - lw);
                if !border {
                    prop_assert_eq!(get(&got, px, py), get(&c, px, py));
                }
            }
        }
    }

    /// P5: each flip, done twice, gives the same canvas.
    #[test]
    fn flips_are_involutions(c in any_canvas()) {
        let mut a = c.clone();
        flip_h(&mut a);
        flip_h(&mut a);
        prop_assert_eq!(&a, &c);
        flip_v(&mut a);
        flip_v(&mut a);
        prop_assert_eq!(&a, &c);
    }

    /// P5: a flip moves each pixel to its mirror place.
    #[test]
    fn flip_h_mirrors(c in canvas()) {
        let mut a = c.clone();
        flip_h(&mut a);
        for y in 0..c.h as i32 {
            for x in 0..c.w as i32 {
                prop_assert_eq!(get(&a, x, y), get(&c, c.w as i32 - 1 - x, y));
            }
        }
    }

    /// P6: four turns give the same canvas. One turn swaps the sides.
    #[test]
    fn four_turns_are_the_identity(c in any_canvas()) {
        let r = rotate90(&c);
        if valid(&c) {
            prop_assert_eq!((r.w, r.h), (c.h, c.w));
        }
        prop_assert_eq!(rotate90(&rotate90(&rotate90(&r))), c);
    }

    /// P7: invert twice gives the same canvas, and alpha never changes.
    #[test]
    fn invert_is_an_involution(c in any_canvas()) {
        let mut a = c.clone();
        invert(&mut a);
        if valid(&c) {
            for i in 0..c.px.len() {
                prop_assert_eq!(a.px[i] >> 24, c.px[i] >> 24);
            }
        }
        invert(&mut a);
        prop_assert_eq!(a, c);
    }

    /// P8: the color eraser changes only `from` pixels in the rectangle, to `to`.
    #[test]
    fn color_replace_changes_only_from_pixels(c in canvas(), x in -3i32..12, y in -3i32..12, w in -2i32..14, h in -2i32..14, from in color(), to in color()) {
        let mut got = c.clone();
        color_replace(&mut got, x, y, w, h, from, to);
        for py in 0..c.h as i32 {
            for px in 0..c.w as i32 {
                let inside = px >= x && px < x + w && py >= y && py < y + h;
                let old = get(&c, px, py);
                let want = if inside && old == from { to } else { old };
                prop_assert_eq!(get(&got, px, py), want);
            }
        }
    }

    /// P9: a clipped rectangle is always inside the canvas.
    #[test]
    fn clip_rect_stays_inside(x in coord(), y in coord(), w in coord(), h in coord(), cw in 0usize..5000, ch in 0usize..5000) {
        let r = clip_rect(x, y, w, h, cw, ch);
        prop_assert!(r.x + r.w <= cw.min(MAX_DIM));
        prop_assert!(r.y + r.h <= ch.min(MAX_DIM));
    }

    /// P10: no function panics, for any canvas and any numbers. An invalid canvas
    /// does not change.
    #[test]
    fn nothing_panics(c in any_canvas(), x in coord(), y in coord(), w in coord(), h in coord(), col in color()) {
        let before = c.clone();
        let mut a = c.clone();
        set(&mut a, x, y, col);
        let _ = get(&a, x, y);
        fill_rect(&mut a, x, y, w, h, col);
        rect_outline(&mut a, x, y, w, h, w, col);
        line(&mut a, x, y, x.wrapping_add(7), y.wrapping_sub(3), h, col);
        flood_fill(&mut a, x, y, col);
        color_replace(&mut a, x, y, w, h, col, WHITE);
        flip_h(&mut a);
        flip_v(&mut a);
        invert(&mut a);
        to_black_white(&mut a);
        let r = rotate90(&a);
        let s = copy_region(&a, x, y, w.rem_euclid(20), h.rem_euclid(20));
        blit(&mut a, &s, y, x, true, col);
        let _ = resize(&r, w.rem_euclid(30) as usize, h.rem_euclid(30) as usize, col);
        let _ = stretch(&a, w.rem_euclid(30) as usize, h.rem_euclid(30) as usize);
        let _ = skew_x(&a, x, col);
        let _ = skew_y(&a, y, col);
        if !valid(&before) {
            prop_assert_eq!(a, before);
        }
    }

    /// Copying a region and pasting it back at the same place changes nothing.
    #[test]
    fn copy_then_blit_is_the_identity(c in canvas(), x in -2i32..12, y in -2i32..12, w in 1i32..12, h in 1i32..12) {
        let s = copy_region(&c, x, y, w, h);
        let mut a = c.clone();
        blit(&mut a, &s, x, y, false, 0);
        prop_assert_eq!(a, c);
    }

    /// Draw Transparent: pixels of the key color in the selection leave the picture as it was.
    #[test]
    fn blit_skips_the_key_color(c in canvas(), s in canvas(), x in -2i32..12, y in -2i32..12, key in color()) {
        let mut a = c.clone();
        blit(&mut a, &s, x, y, true, key);
        for sy in 0..s.h as i32 {
            for sx in 0..s.w as i32 {
                let p = get(&s, sx, sy);
                if p == key || p >> 24 == 0 {
                    prop_assert_eq!(get(&a, x + sx, y + sy), get(&c, x + sx, y + sy));
                }
            }
        }
    }

    /// Resize keeps the overlap at the top-left and fills the rest with `bg`.
    #[test]
    fn resize_keeps_the_top_left(c in canvas(), w in 1usize..15, h in 1usize..15, bg in color()) {
        let r = resize(&c, w, h, bg);
        prop_assert_eq!((r.w, r.h), (w, h));
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let want = if (x as usize) < c.w && (y as usize) < c.h { get(&c, x, y) } else { bg };
                prop_assert_eq!(get(&r, x, y), want);
            }
        }
    }

    /// Stretch to the same size, or skew by 0, gives the same canvas.
    #[test]
    fn neutral_stretch_and_skew_change_nothing(c in canvas(), bg in color()) {
        prop_assert_eq!(&stretch(&c, c.w, c.h), &c);
        prop_assert_eq!(&skew_x(&c, 0, bg), &c);
        prop_assert_eq!(&skew_y(&c, 0, bg), &c);
    }

    /// Stretch gives the asked size. Every pixel color comes from the source.
    #[test]
    fn stretch_keeps_the_size_and_colors(c in canvas(), nw in 1usize..30, nh in 1usize..30) {
        let s = stretch(&c, nw, nh);
        prop_assert_eq!((s.w, s.h), (nw, nh));
        for &p in &s.px {
            prop_assert!(c.px.contains(&p));
        }
    }

    /// Skew keeps every source pixel, once each, and the new pixels are `bg`.
    #[test]
    fn skew_keeps_every_pixel(c in canvas(), num in -3000i32..3000) {
        let bg = BLUE;
        for s in [skew_x(&c, num, bg), skew_y(&c, num, bg)] {
            prop_assert!(valid(&s));
            let kept = s.px.iter().filter(|&&p| p != bg).count();
            prop_assert_eq!(kept, c.px.iter().filter(|&&p| p != bg).count());
        }
    }

    /// Black and white gives only black or white, with the same alpha.
    #[test]
    fn black_white_gives_two_colors(c in canvas()) {
        let mut a = c.clone();
        to_black_white(&mut a);
        for i in 0..c.px.len() {
            prop_assert!(a.px[i] & RGB_MASK == 0 || a.px[i] & RGB_MASK == RGB_MASK);
            prop_assert_eq!(a.px[i] >> 24, c.px[i] >> 24);
        }
    }
}
