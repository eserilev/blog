//! Pixel logic for Paint (spec 11.9). No I/O, no floats, no `unsafe`.
//!
//! Aeneas verifies this crate (spec 7.6, theorems P1–P10), so the code keeps to the
//! same Rust subset as `logbook-core`: integers, structs, `while` loops with indices,
//! `Option`, and `Vec`. No iterators, closures, floats, or `std::cmp` helpers.
//!
//! Every public function is total. It accepts any input, also a [`Canvas`] whose
//! fields do not agree, and it never panics (P10). A function does not change a
//! canvas that is not [`valid`].
//!
//! A pixel is one `u32` in little-endian RGBA order: `r | g << 8 | b << 16 | a << 24`.
//! The bytes of `px` are then the same bytes as a browser `ImageData`.

// Aeneas reads these forms best as written. The casts are checked by hand: every
// coordinate is clamped to ±`COORD_LIMIT` and every size to `MAX_DIM` first.
#![allow(
    clippy::manual_range_contains,
    clippy::len_zero,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::manual_swap,
    clippy::manual_clamp
)]

/// The largest width or height of a canvas, in pixels.
pub const MAX_DIM: usize = 4096;
/// Coordinates are clamped to `-COORD_LIMIT..=COORD_LIMIT`, so sums never overflow.
pub const COORD_LIMIT: i32 = 65_536;
/// The widest line or brush, in pixels.
pub const MAX_WIDTH: i32 = 64;
/// Skew factors are fractions with this denominator: `num / SKEW_DEN = tan(angle)`.
pub const SKEW_DEN: i32 = 1000;
/// The largest skew numerator: `tan(89°) × 1000`, rounded down.
pub const SKEW_MAX: i32 = 57_289;
/// A pixel with alpha 0. Blits skip it.
pub const TRANSPARENT: u32 = 0;
/// `invert` flips these bits: red, green, and blue. Alpha stays.
pub const RGB_MASK: u32 = 0x00FF_FFFF;

/// A picture: `w × h` pixels, row by row. It is [`valid`] if `px.len() == w * h`
/// and both sides are in `1..=MAX_DIM`.
#[derive(Debug, PartialEq, Eq)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

/// `Clone` is written out by hand, as in `logbook-core`: Aeneas models `Vec::clone`.
impl Clone for Canvas {
    fn clone(&self) -> Self {
        Canvas {
            w: self.w,
            h: self.h,
            px: self.px.clone(),
        }
    }
}

/// A rectangle inside a canvas. [`clip_rect`] makes it (P9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// True if the fields of `c` agree (see [`Canvas`]).
#[must_use]
pub fn valid(c: &Canvas) -> bool {
    c.w >= 1 && c.h >= 1 && c.w <= MAX_DIM && c.h <= MAX_DIM && c.px.len() == c.w * c.h
}

// ---- Small helpers. Plain code, because Aeneas has no model for `std::cmp`. ----

fn min_i64(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

fn max_i64(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}

fn abs_i32(v: i32) -> i32 {
    // Callers pass clamped values, so `-v` cannot overflow.
    if v < 0 { -v } else { v }
}

/// `v` clamped to `-COORD_LIMIT..=COORD_LIMIT`.
#[must_use]
pub fn clamp_coord(v: i32) -> i32 {
    if v < -COORD_LIMIT {
        -COORD_LIMIT
    } else if v > COORD_LIMIT {
        COORD_LIMIT
    } else {
        v
    }
}

/// A rectangle side clamped to `0..=2 × COORD_LIMIT`.
fn clamp_span(v: i32) -> i32 {
    if v < 0 {
        0
    } else if v > 2 * COORD_LIMIT {
        2 * COORD_LIMIT
    } else {
        v
    }
}

/// A line or brush width clamped to `1..=MAX_WIDTH`.
fn clamp_width(v: i32) -> i32 {
    if v < 1 {
        1
    } else if v > MAX_WIDTH {
        MAX_WIDTH
    } else {
        v
    }
}

/// A canvas side clamped to `1..=MAX_DIM`.
#[must_use]
pub fn clamp_dim(n: usize) -> usize {
    if n < 1 {
        1
    } else if n > MAX_DIM {
        MAX_DIM
    } else {
        n
    }
}

/// `n` copies of `v`.
fn filled(n: usize, v: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        out.push(v);
        i += 1;
    }
    out
}

/// `n` zeros.
fn zeros(n: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        out.push(0);
        i += 1;
    }
    out
}

// ---- Canvases and pixels ----

/// A `w × h` canvas of one color. The sides are clamped to `1..=MAX_DIM`.
#[must_use]
pub fn new_canvas(w: usize, h: usize, color: u32) -> Canvas {
    let w = clamp_dim(w);
    let h = clamp_dim(h);
    Canvas {
        w,
        h,
        px: filled(w * h, color),
    }
}

/// The index of pixel `(x, y)` in `px`, or `None` if it is outside the canvas.
#[must_use]
pub fn index(c: &Canvas, x: i32, y: i32) -> Option<usize> {
    if !valid(c) || x < 0 || y < 0 {
        return None;
    }
    let xu = x as usize;
    let yu = y as usize;
    if xu >= c.w || yu >= c.h {
        return None;
    }
    Some(yu * c.w + xu)
}

/// The pixel at `(x, y)`, or [`TRANSPARENT`] outside the canvas.
#[must_use]
pub fn get(c: &Canvas, x: i32, y: i32) -> u32 {
    match index(c, x, y) {
        Some(i) => c.px[i],
        None => TRANSPARENT,
    }
}

/// Sets the pixel at `(x, y)`. Does nothing outside the canvas.
pub fn set(c: &mut Canvas, x: i32, y: i32, color: u32) {
    if let Some(i) = index(c, x, y) {
        c.px[i] = color;
    }
}

/// The part of the rectangle `x, y, w, h` that is inside a `cw × ch` canvas (P9).
/// A negative or zero side gives an empty rectangle.
#[must_use]
pub fn clip_rect(x: i32, y: i32, w: i32, h: i32, cw: usize, ch: usize) -> Rect {
    let x = i64::from(clamp_coord(x));
    let y = i64::from(clamp_coord(y));
    let w = i64::from(clamp_span(w));
    let h = i64::from(clamp_span(h));
    // Clamp before the cast: a `usize` above `i64::MAX` would wrap to a negative value.
    let cw = (if cw < MAX_DIM { cw } else { MAX_DIM }) as i64;
    let ch = (if ch < MAX_DIM { ch } else { MAX_DIM }) as i64;
    let x0 = min_i64(max_i64(x, 0), cw);
    let y0 = min_i64(max_i64(y, 0), ch);
    let x1 = min_i64(max_i64(x + w, x0), cw);
    let y1 = min_i64(max_i64(y + h, y0), ch);
    Rect {
        x: x0 as usize,
        y: y0 as usize,
        w: (x1 - x0) as usize,
        h: (y1 - y0) as usize,
    }
}

/// Fills the part of the rectangle `x, y, w, h` that is inside the canvas.
pub fn fill_rect(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, color: u32) {
    if !valid(c) {
        return;
    }
    let r = clip_rect(x, y, w, h, c.w, c.h);
    let mut yy = r.y;
    while yy < r.y + r.h {
        let mut xx = r.x;
        while xx < r.x + r.w {
            c.px[yy * c.w + xx] = color;
            xx += 1;
        }
        yy += 1;
    }
}

/// Draws the border of the rectangle `x, y, w, h`, `lw` pixels thick (P4).
/// Only pixels of the rectangle that are less than `lw` from one of its edges change.
pub fn rect_outline(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, lw: i32, color: u32) {
    let x = clamp_coord(x);
    let y = clamp_coord(y);
    let w = clamp_span(w);
    let h = clamp_span(h);
    let lw = clamp_width(lw);
    if w == 0 || h == 0 {
        return;
    }
    // A border thicker than the rectangle stays inside the rectangle.
    let tx = if lw < w { lw } else { w };
    let ty = if lw < h { lw } else { h };
    fill_rect(c, x, y, w, ty, color);
    fill_rect(c, x, y + h - ty, w, ty, color);
    fill_rect(c, x, y, tx, h, color);
    fill_rect(c, x + w - tx, y, tx, h, color);
}

/// The pixels of the line from `(x0, y0)` to `(x1, y1)`: Bresenham's algorithm (P3).
///
/// The points are clamped first. The result starts at the first point and ends at
/// the second. It has `max(|dx|, |dy|) + 1` points, each one a neighbor (8-way) of
/// the one before, and all of them in the bounding box of the two points.
#[must_use]
pub fn line_points(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let x0 = clamp_coord(x0);
    let y0 = clamp_coord(y0);
    let x1 = clamp_coord(x1);
    let y1 = clamp_coord(y1);
    let dx = abs_i32(x1 - x0);
    let dy = -abs_i32(y1 - y0);
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let n = if dx > -dy { dx } else { -dy };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    let mut out = Vec::new();
    out.push((x, y));
    let mut k = 0;
    while k < n {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        out.push((x, y));
        k += 1;
    }
    out
}

/// Draws a line `width` pixels wide: a `width × width` square at each point of
/// [`line_points`].
pub fn line(c: &mut Canvas, x0: i32, y0: i32, x1: i32, y1: i32, width: i32, color: u32) {
    if !valid(c) {
        return;
    }
    let wd = clamp_width(width);
    let off = (wd - 1) / 2;
    let pts = line_points(x0, y0, x1, y1);
    let mut i = 0;
    while i < pts.len() {
        let (px, py) = pts[i];
        fill_rect(c, px - off, py - off, wd, wd, color);
        i += 1;
    }
}

/// Fill With Color (P1, P2). Changes exactly the pixels that have the color of
/// `(x, y)` and connect to it through 4-way neighbors of that color. Nothing else.
///
/// Each pixel takes the new color when it goes on the stack, so it goes on the stack
/// at most once. The stack therefore never holds more than `w × h` entries, and the
/// loop ends.
pub fn flood_fill(c: &mut Canvas, x: i32, y: i32, color: u32) {
    let Some(start) = index(c, x, y) else {
        return;
    };
    let target = c.px[start];
    if target == color {
        return;
    }
    let w = c.w;
    let n = c.px.len();
    let mut stack = zeros(n);
    c.px[start] = color;
    stack[0] = start;
    let mut top = 1;
    while top > 0 {
        top -= 1;
        let i = stack[top];
        let col = i % w;
        // Each neighbor index is computed once. rustc turns a repeated `i - 1` into an
        // unchecked subtraction, and Aeneas cannot translate that.
        if col > 0 {
            let j = i - 1;
            if c.px[j] == target {
                c.px[j] = color;
                stack[top] = j;
                top += 1;
            }
        }
        if col + 1 < w {
            let j = i + 1;
            if c.px[j] == target {
                c.px[j] = color;
                stack[top] = j;
                top += 1;
            }
        }
        if i >= w {
            let j = i - w;
            if c.px[j] == target {
                c.px[j] = color;
                stack[top] = j;
                top += 1;
            }
        }
        let j = i + w;
        if j < n && c.px[j] == target {
            c.px[j] = color;
            stack[top] = j;
            top += 1;
        }
    }
}

/// The color eraser (P8). In the part of the rectangle inside the canvas, pixels of
/// color `from` become `to`. No other pixel changes.
pub fn color_replace(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, from: u32, to: u32) {
    if !valid(c) {
        return;
    }
    let r = clip_rect(x, y, w, h, c.w, c.h);
    let mut yy = r.y;
    while yy < r.y + r.h {
        let mut xx = r.x;
        while xx < r.x + r.w {
            let i = yy * c.w + xx;
            if c.px[i] == from {
                c.px[i] = to;
            }
            xx += 1;
        }
        yy += 1;
    }
}

// ---- Whole-picture operations ----

/// Flip horizontal (P5). Doing it twice gives the same canvas.
pub fn flip_h(c: &mut Canvas) {
    if !valid(c) {
        return;
    }
    let mut y = 0;
    while y < c.h {
        let row = y * c.w;
        let mut a = 0;
        let mut b = c.w - 1;
        while a < b {
            let t = c.px[row + a];
            c.px[row + a] = c.px[row + b];
            c.px[row + b] = t;
            a += 1;
            b -= 1;
        }
        y += 1;
    }
}

/// Flip vertical (P5). Doing it twice gives the same canvas.
pub fn flip_v(c: &mut Canvas) {
    if !valid(c) {
        return;
    }
    let mut a = 0;
    let mut b = c.h - 1;
    while a < b {
        let mut x = 0;
        while x < c.w {
            let i = a * c.w + x;
            let j = b * c.w + x;
            let t = c.px[i];
            c.px[i] = c.px[j];
            c.px[j] = t;
            x += 1;
        }
        a += 1;
        b -= 1;
    }
}

/// Rotate 90° clockwise (P6). The sides swap. Four turns give the same canvas.
#[must_use]
pub fn rotate90(c: &Canvas) -> Canvas {
    if !valid(c) {
        return c.clone();
    }
    let nw = c.h;
    let nh = c.w;
    let mut out = Canvas {
        w: nw,
        h: nh,
        px: filled(nw * nh, TRANSPARENT),
    };
    let mut y = 0;
    while y < c.h {
        let mut x = 0;
        while x < c.w {
            out.px[x * nw + (c.h - 1 - y)] = c.px[y * c.w + x];
            x += 1;
        }
        y += 1;
    }
    out
}

/// Invert Colors (P7). Red, green, and blue flip. Alpha stays. Twice is the identity.
pub fn invert(c: &mut Canvas) {
    if !valid(c) {
        return;
    }
    let mut i = 0;
    while i < c.px.len() {
        c.px[i] ^= RGB_MASK;
        i += 1;
    }
}

/// Black and white: each pixel becomes black or white by its brightness. Alpha stays.
pub fn to_black_white(c: &mut Canvas) {
    if !valid(c) {
        return;
    }
    let mut i = 0;
    while i < c.px.len() {
        let p = c.px[i];
        let r = p & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = (p >> 16) & 0xFF;
        let lum = (r * 30 + g * 59 + b * 11) / 100;
        let a = p & 0xFF00_0000;
        c.px[i] = if lum > 127 { a | RGB_MASK } else { a };
        i += 1;
    }
}

/// A copy of the rectangle `x, y, w, h`. Its sides are clamped to `1..=MAX_DIM`.
/// Pixels outside the source canvas are [`TRANSPARENT`].
#[must_use]
pub fn copy_region(c: &Canvas, x: i32, y: i32, w: i32, h: i32) -> Canvas {
    let w = if w < 1 { 1 } else { clamp_dim(w as usize) };
    let h = if h < 1 { 1 } else { clamp_dim(h as usize) };
    let x = clamp_coord(x);
    let y = clamp_coord(y);
    let mut out = Canvas {
        w,
        h,
        px: filled(w * h, TRANSPARENT),
    };
    let mut yy = 0;
    while yy < h {
        let mut xx = 0;
        while xx < w {
            out.px[yy * w + xx] = get(c, x + xx as i32, y + yy as i32);
            xx += 1;
        }
        yy += 1;
    }
    out
}

/// Draws `src` onto `dst` with its top-left corner at `(x, y)`. Pixels with alpha 0
/// are skipped. If `use_key` is true, pixels of color `key` are skipped too: this is
/// "Draw Transparent".
pub fn blit(dst: &mut Canvas, src: &Canvas, x: i32, y: i32, use_key: bool, key: u32) {
    if !valid(dst) || !valid(src) {
        return;
    }
    let x = clamp_coord(x);
    let y = clamp_coord(y);
    let mut sy = 0;
    while sy < src.h {
        let mut sx = 0;
        while sx < src.w {
            let p = src.px[sy * src.w + sx];
            if (p >> 24) != 0 && !(use_key && p == key) {
                set(dst, x + sx as i32, y + sy as i32, p);
            }
            sx += 1;
        }
        sy += 1;
    }
}

/// A `w × h` canvas filled with `bg`, with the old picture at its top-left corner.
#[must_use]
pub fn resize(c: &Canvas, w: usize, h: usize, bg: u32) -> Canvas {
    let mut out = new_canvas(w, h, bg);
    if !valid(c) {
        return out;
    }
    let mw = if c.w < out.w { c.w } else { out.w };
    let mh = if c.h < out.h { c.h } else { out.h };
    let mut y = 0;
    while y < mh {
        let mut x = 0;
        while x < mw {
            out.px[y * out.w + x] = c.px[y * c.w + x];
            x += 1;
        }
        y += 1;
    }
    out
}

/// Stretch to `nw × nh` with the nearest pixel. The same size gives the same canvas.
#[must_use]
pub fn stretch(c: &Canvas, nw: usize, nh: usize) -> Canvas {
    if !valid(c) {
        return c.clone();
    }
    let nw = clamp_dim(nw);
    let nh = clamp_dim(nh);
    let mut out = Canvas {
        w: nw,
        h: nh,
        px: filled(nw * nh, TRANSPARENT),
    };
    let mut y = 0;
    while y < nh {
        let sy = y * c.h / nh;
        let mut x = 0;
        while x < nw {
            let sx = x * c.w / nw;
            out.px[y * nw + x] = c.px[sy * c.w + sx];
            x += 1;
        }
        y += 1;
    }
    out
}

/// Skew horizontal: row `y` moves `y × num / SKEW_DEN` pixels to the right. The
/// canvas grows to fit, up to `MAX_DIM`. New pixels are `bg`. `num == 0` is the identity.
#[must_use]
pub fn skew_x(c: &Canvas, num: i32, bg: u32) -> Canvas {
    if !valid(c) {
        return c.clone();
    }
    let num = i64::from(if num < -SKEW_MAX {
        -SKEW_MAX
    } else if num > SKEW_MAX {
        SKEW_MAX
    } else {
        num
    });
    let den = i64::from(SKEW_DEN);
    let last = (c.h as i64 - 1) * num / den;
    let extra = if last < 0 { -last } else { last };
    let base = if num < 0 { extra } else { 0 };
    let mut out = new_canvas(c.w + extra as usize, c.h, bg);
    let mut y = 0;
    while y < c.h {
        let shift = y as i64 * num / den + base;
        let mut x = 0;
        while x < c.w {
            let nx = x as i64 + shift;
            if nx >= 0 && nx < out.w as i64 {
                out.px[y * out.w + nx as usize] = c.px[y * c.w + x];
            }
            x += 1;
        }
        y += 1;
    }
    out
}

/// Skew vertical: column `x` moves `x × num / SKEW_DEN` pixels down. See [`skew_x`].
#[must_use]
pub fn skew_y(c: &Canvas, num: i32, bg: u32) -> Canvas {
    if !valid(c) {
        return c.clone();
    }
    let num = i64::from(if num < -SKEW_MAX {
        -SKEW_MAX
    } else if num > SKEW_MAX {
        SKEW_MAX
    } else {
        num
    });
    let den = i64::from(SKEW_DEN);
    let last = (c.w as i64 - 1) * num / den;
    let extra = if last < 0 { -last } else { last };
    let base = if num < 0 { extra } else { 0 };
    let mut out = new_canvas(c.w, c.h + extra as usize, bg);
    let mut x = 0;
    while x < c.w {
        let shift = x as i64 * num / den + base;
        let mut y = 0;
        while y < c.h {
            let ny = y as i64 + shift;
            if ny >= 0 && ny < out.h as i64 {
                out.px[ny as usize * out.w + x] = c.px[y * c.w + x];
            }
            y += 1;
        }
        x += 1;
    }
    out
}

#[cfg(test)]
mod tests;
