use ab_glyph::{Font, FontVec, Point, PxScale, ScaleFont};
use std::collections::HashMap;

/// Atlas texture dimensions (square).
const ATLAS_SIZE: u32 = 256;
/// Glyph slot size within the atlas.
const GLYPH_SIZE: u32 = 16;
/// Number of glyph slots per axis.
const ATLAS_COLS: u32 = ATLAS_SIZE / GLYPH_SIZE;
/// Maximum number of glyphs the atlas can hold.
const ATLAS_CAPACITY: usize = (ATLAS_COLS * ATLAS_COLS) as usize;

/// Rasterized glyph metadata.
pub struct GlyphInfo {
    /// Slot index in the atlas grid (0..255).
    pub slot: u32,
    /// Bounding box offset within the slot (pixels from top-left).
    pub offset_x: f32,
    pub offset_y: f32,
    /// Rasterized glyph dimensions.
    pub width: u32,
    pub height: u32,
}

/// Dynamic glyph atlas backed by a 256x256 R8 texture.
///
/// Glyphs are rasterized on demand via `ab_glyph` and cached in a
/// 16×16 grid.  The atlas is pre-populated with printable ASCII
/// characters on construction.
pub struct GlyphAtlas {
    font: FontVec,
    scale: PxScale,
    /// RGBA8 texture data (only R channel is meaningful).
    pixels: Vec<u8>,
    /// char → glyph info lookup.
    cache: HashMap<char, GlyphInfo>,
    /// Next free slot index.
    next_slot: u32,
    /// Dirty flag: true when pixels have changed since last upload.
    dirty: bool,
    /// Horizontal advance width for monospace cell sizing.
    advance_width: f32,
}

impl GlyphAtlas {
    /// Create a new atlas from raw font bytes at the given pixel size.
    pub fn new(font_bytes: &[u8], font_size: f32) -> Self {
        let font = FontVec::try_from_vec(font_bytes.to_vec()).expect("Failed to parse font");
        let scale = PxScale::from(font_size);
        // Measure advance width from a representative glyph ('M').
        let scaled = font.as_scaled(scale);
        let m_id = font.glyph_id('M');
        let advance_width = scaled.h_advance(m_id);
        let mut atlas = Self {
            font,
            scale,
            pixels: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            cache: HashMap::new(),
            next_slot: 0,
            dirty: true,
            advance_width,
        };
        atlas.populate_ascii();
        atlas
    }

    /// Ensure a character is present in the atlas, rasterizing if needed.
    pub fn ensure_char(&mut self, ch: char) -> &GlyphInfo {
        if !self.cache.contains_key(&ch) {
            self.rasterize(ch);
        }
        &self.cache[&ch]
    }

    /// Returns true if the pixel data has been modified since the last upload.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the atlas as clean (after GPU upload).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Raw pixel data (256×256 R8).
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Atlas texture size in pixels.
    pub fn size(&self) -> u32 {
        ATLAS_SIZE
    }

    /// Glyph slot size.
    pub fn glyph_size(&self) -> u32 {
        GLYPH_SIZE
    }

    /// Horizontal advance width for monospace cell sizing.
    pub fn advance_width(&self) -> f32 {
        self.advance_width
    }

    // ── internal ──

    fn populate_ascii(&mut self) {
        for ch in ' '..='~' {
            self.rasterize(ch);
        }
        // Common box-drawing and block characters
        for ch in [
            '│', '─', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '█', '▌', '▐', '▄', '▀', '░',
            '▒', '▓',
        ] {
            self.rasterize(ch);
        }
    }

    fn rasterize(&mut self, ch: char) {
        if self.cache.contains_key(&ch) {
            return;
        }
        if self.next_slot >= ATLAS_CAPACITY as u32 {
            // Evict: reset atlas (crude but simple for a terminal).
            self.pixels.fill(0);
            self.cache.clear();
            self.next_slot = 0;
        }

        let slot = self.next_slot;
        self.next_slot += 1;

        let glyph_id = self.font.glyph_id(ch);
        let scaled = self.font.as_scaled(self.scale);

        // Get glyph outline and rasterize.
        let glyph = ab_glyph::Glyph {
            id: glyph_id,
            scale: self.scale,
            position: Point { x: 0.0, y: 0.0 },
        };

        let (glyph_w, glyph_h, bitmap) =
            if let Some(outlined) = self.font.outline_glyph(glyph.clone()) {
                let bounds = outlined.px_bounds();
                let w = bounds.width().ceil() as u32;
                let h = bounds.height().ceil() as u32;
                let mut bmp = vec![0.0f32; (w * h) as usize];
                outlined.draw(|x, y, v| {
                    if x < w && y < h {
                        bmp[(y * w + x) as usize] = v;
                    }
                });
                (w, h, bmp)
            } else {
                // No outline (space, etc.) — use advance width only.
                let adv = scaled.h_advance(glyph_id);
                (adv.ceil() as u32, 0, Vec::new())
            };

        // Copy into atlas slot, centered vertically.
        let col = slot % ATLAS_COLS;
        let row = slot / ATLAS_COLS;
        let slot_x = col * GLYPH_SIZE;
        let slot_y = row * GLYPH_SIZE;

        // Vertical offset to center glyph in slot.
        let y_off = if glyph_h < GLYPH_SIZE {
            (GLYPH_SIZE - glyph_h) / 2
        } else {
            0
        };
        // Horizontal offset (for proportional fonts in monospace cells).
        let x_off = if glyph_w < GLYPH_SIZE {
            (GLYPH_SIZE - glyph_w) / 2
        } else {
            0
        };

        let copy_w = glyph_w.min(GLYPH_SIZE);
        let copy_h = glyph_h.min(GLYPH_SIZE);

        for gy in 0..copy_h {
            for gx in 0..copy_w {
                let src_idx = (gy * glyph_w + gx) as usize;
                let dst_x = slot_x + x_off + gx;
                let dst_y = slot_y + y_off + gy;
                if dst_x < ATLAS_SIZE && dst_y < ATLAS_SIZE {
                    let dst_idx = (dst_y * ATLAS_SIZE + dst_x) as usize;
                    let alpha = if src_idx < bitmap.len() {
                        bitmap[src_idx]
                    } else {
                        0.0
                    };
                    self.pixels[dst_idx] = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }

        self.cache.insert(
            ch,
            GlyphInfo {
                slot,
                offset_x: x_off as f32,
                offset_y: y_off as f32,
                width: glyph_w,
                height: glyph_h,
            },
        );
        self.dirty = true;
    }
}
