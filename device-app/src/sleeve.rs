//! Sleeve bitmaps: the 1bpp square a record shows.
//!
//! ## Packing convention
//!
//! Stored art is packed so that the display, which decodes row-major, shows
//! the image upright. Measured from on-device renders (see
//! `docs/art/README.md`): the screen shows the buffer's row-major decode
//! rotated 90 degrees clockwise, so the packer pre-rotates counter-clockwise.
//! For an image pixel `(x, y)` of an `N x N` sleeve:
//!
//! ```text
//! bit = (N - 1 - x) * N + y      byte = bit / 8, mask = 0x80 >> (bit % 8)
//! ```
//!
//! A bit set means white. The host-side packer (`scripts/sleeve.py`) emits
//! exactly this, and `docs/art/test-pattern.bin` exists to confirm it on
//! screen rather than on trust.
//!
//! ## Working in stored space
//!
//! [`decimate`] never undoes the rotation. Writing the stored index as
//! `k = i * N + j`, the rotation is just a relabelling of `(x, y)` as
//! `(i, j)`, and it maps 2x2 blocks to 2x2 blocks. So a box filter applied in
//! `(i, j)` produces a correctly rotated half-size sleeve in the same
//! convention, with no unpacking and no second buffer.

use alloc::vec::Vec;

/// Turn canonical sleeve bytes into the buffer NBGL should draw.
///
/// The stored (and hashed) bytes are exactly what `scripts/sleeve.py` emits: a
/// set bit is lit art, per that tool's `1 = white` convention. But this 1bpp
/// path renders a set bit as *black* (measured: `scripts/check-packing.py`,
/// correlation 1.0 with `invert`). So to put white art on a black ground, the
/// way every validated preview looks, the device inverts the bits at render
/// time. The canonical bytes, and therefore the certificate's sleeve hash, are
/// left untouched; only this display copy is flipped.
pub fn to_display(canonical: &[u8]) -> Vec<u8> {
    canonical.iter().map(|b| !b).collect()
}

/// Read pixel `(i, j)` in stored space from a packed 1bpp square.
#[inline]
fn get_bit(data: &[u8], n: usize, i: usize, j: usize) -> bool {
    let k = i * n + j;
    let byte = k >> 3;
    byte < data.len() && data[byte] & (0x80 >> (k & 7)) != 0
}

/// Set pixel `(i, j)` in stored space.
#[inline]
fn set_bit(data: &mut [u8], n: usize, i: usize, j: usize) {
    let k = i * n + j;
    let byte = k >> 3;
    if byte < data.len() {
        data[byte] |= 0x80 >> (k & 7);
    }
}

/// Halve a packed 1bpp square by 2x2 box filter: a block is lit when at least
/// two of its four pixels are. Allocates only the (quarter-size) result, so a
/// grid of thumbnails costs a few hundred bytes each rather than a second
/// full-size copy.
pub fn decimate(src: &[u8], n: usize) -> Vec<u8> {
    let half = n / 2;
    let mut out = alloc::vec![0u8; half * half / 8];
    for i in 0..half {
        for j in 0..half {
            let lit = get_bit(src, n, 2 * i, 2 * j) as u8
                + get_bit(src, n, 2 * i + 1, 2 * j) as u8
                + get_bit(src, n, 2 * i, 2 * j + 1) as u8
                + get_bit(src, n, 2 * i + 1, 2 * j + 1) as u8;
            if lit >= 2 {
                set_bit(&mut out, half, i, j);
            }
        }
    }
    out
}

/// The label art a record falls back to: when it holds no sleeve, or when the
/// stored sleeve's hash does not match the one in the certificate.
///
/// Generated from the album id rather than picked from a set of compiled
/// bitmaps. Shipping the alternatives as glyphs cost tens of kilobytes of
/// flash, which on this app competes directly with the size of the *real*
/// sleeve an artist can store (see `docs/protocol.md`). Computing it also
/// makes every album distinct instead of one of eight.
///
/// A vinyl record seen face-on: a black disc on a light ground, ringed by a
/// solid rim, cut by fine grooves, and centred on a solid label with a punched
/// spindle hole. Matches the aesthetic of `glyphs/vinyl_64x64.gif`. Reads as a
/// record, not a target: the vinyl body stays mostly black, and the grooves are
/// single-pixel rings spaced `pitch` apart, never the equal light/dark bands
/// that made the old placeholder look like radar.
///
/// The groove pitch and the label radius are derived from the album id, and one
/// id bit adds a thin ring inside the label, so no two editions render the same
/// face while every one still reads as a record.
///
/// Returns canonical bytes (a set bit is white art, per the module's polarity
/// note); the caller inverts through [`to_display`] before drawing.
pub fn fallback_sleeve(n: usize, album_id: &[u8; 32]) -> Vec<u8> {
    let mut out = alloc::vec![0u8; n * n / 8];
    let seed = album_id[0];
    let centre = (n / 2) as i32;
    let radius = centre - 2;
    // Groove pitch and label size vary with the id, within ranges that keep the
    // record legible once decimated to a library thumbnail.
    let pitch = 5 + (seed & 3) as i32; // 5..8 px between grooves
    let label_r = radius / 3 + ((seed >> 2) & 3) as i32 * 2;
    let hole_r = 3;
    // One id bit splits the label with a thin groove, a subtle per-album detail.
    let label_ring = (seed >> 4) & 1 == 1;
    let ring_r = label_r / 2;

    for i in 0..n {
        for j in 0..n {
            let dx = i as i32 - centre;
            let dy = j as i32 - centre;
            let d2 = dx * dx + dy * dy;
            // Integer distance, good enough at this scale and free of floats,
            // which are banned on this target.
            let mut d = 0i32;
            while (d + 1) * (d + 1) <= d2 {
                d += 1;
            }
            // In canonical space a set bit is white: the ground and the label
            // are white, the vinyl body is left black with fine white grooves.
            let white = if d > radius {
                true // light ground around the disc
            } else if d <= hole_r {
                false // spindle hole
            } else if d <= label_r {
                !(label_ring && d == ring_r) // solid label, maybe one groove
            } else if d <= label_r + 1 || d >= radius - 1 {
                false // crisp black gap round the label, solid black outer rim
            } else {
                d % pitch == 0 // fine white grooves on the black vinyl
            };
            if white {
                set_bit(&mut out, n, i, j);
            }
        }
    }
    out
}
