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
/// The design is a record seen face-on: an outer rim, grooves whose spacing
/// is set by the id, and a centre label with a spindle hole. Two id bits
/// break the symmetry so two albums never look alike.
pub fn fallback_sleeve(n: usize, album_id: &[u8; 32]) -> Vec<u8> {
    let mut out = alloc::vec![0u8; n * n / 8];
    let seed = album_id[0];
    let centre = (n / 2) as i32;
    let radius = centre - 2;
    // Groove pitch and label size vary with the id, within ranges that stay
    // legible once the sleeve is decimated to a thumbnail.
    let pitch = 4 + (seed & 3) as i32 * 2;
    let label_r = radius / 3 + ((seed >> 2) & 3) as i32 * 2;
    let notch = (seed >> 4) & 3;

    for i in 0..n {
        for j in 0..n {
            let dx = i as i32 - centre;
            let dy = j as i32 - centre;
            let d2 = dx * dx + dy * dy;
            if d2 > radius * radius {
                continue; // outside the disc: leave it dark
            }
            // Integer distance, good enough at this scale and free of floats,
            // which are banned on this target.
            let mut d = 0i32;
            while (d + 1) * (d + 1) <= d2 {
                d += 1;
            }
            let lit = if d <= label_r {
                // Centre label: solid, with a spindle hole punched out.
                d > 3
            } else if d >= radius - 2 {
                true // outer rim
            } else {
                // Grooves, interrupted in one quadrant so the art is
                // asymmetric and the id is visible at a glance.
                let quadrant = ((dx > 0) as u8) | (((dy > 0) as u8) << 1);
                quadrant != notch && (d / pitch) % 2 == 0
            };
            if lit {
                set_bit(&mut out, n, i, j);
            }
        }
    }
    out
}
