use collections::HashMap;
use cosmic_text::Font;
use freetype_sys as ft;
use std::{ptr, sync::Arc};

pub(super) struct HintedMetrics {
    library: ft::FT_Library,
    face: ft::FT_Face,
    _font: Arc<Font>,
    advances: HashMap<(u32, u16), (f32, f32)>,
}

// FreeType handles are private and only accessed through &mut self while the
// owning CosmicTextSystemState's write lock is held. The font bytes stay alive.
unsafe impl Send for HintedMetrics {}
unsafe impl Sync for HintedMetrics {}

impl HintedMetrics {
    pub(super) fn new(font: Arc<Font>, index: u32, weight: u16) -> Option<Self> {
        let length = font.data().len().try_into().ok()?;
        let mut library = ptr::null_mut();
        let mut face = ptr::null_mut();
        unsafe {
            if ft::FT_Init_FreeType(&mut library) != 0 {
                return None;
            }
            if ft::FT_New_Memory_Face(
                library,
                font.data().as_ptr(),
                length,
                index.into(),
                &mut face,
            ) != 0
            {
                ft::FT_Done_FreeType(library);
                return None;
            }
        }
        let metrics = Self {
            library,
            face,
            _font: font,
            advances: HashMap::default(),
        };
        unsafe {
            if !ft::FT_IS_SCALABLE(face) {
                return None;
            }
            if ft::FT_HAS_MULTIPLE_MASTERS(face) {
                let mut axes = ptr::null_mut();
                if ft::FT_Get_MM_Var(face, &mut axes) != 0 {
                    return None;
                }
                let coords: Vec<_> =
                    std::slice::from_raw_parts((*axes).axis, (*axes).num_axis as usize)
                        .iter()
                        .map(|axis| {
                            if axis.tag == u32::from_be_bytes(*b"wght").into() {
                                ((weight as ft::FT_Fixed) << 16).clamp(axis.minimum, axis.maximum)
                            } else {
                                axis.def
                            }
                        })
                        .collect();
                let result =
                    ft::FT_Set_Var_Design_Coordinates(face, coords.len() as u32, coords.as_ptr());
                ft::FT_Done_MM_Var(library, axes);
                if result != 0 {
                    return None;
                }
            }
        }
        Some(metrics)
    }

    pub(super) fn advances(&mut self, size: f32, glyph: u16) -> Option<(f32, f32)> {
        if !size.is_finite() || size <= 0.0 {
            return None;
        }
        let key = (size.to_bits(), glyph);
        if let Some(advances) = self.advances.get(&key) {
            return Some(*advances);
        }
        unsafe {
            if ft::FT_Set_Char_Size(
                self.face,
                0,
                (size * 64.0).round() as ft::FT_F26Dot6,
                72,
                72,
            ) != 0
                || ft::FT_Load_Glyph(
                    self.face,
                    glyph.into(),
                    ft::FT_LOAD_DEFAULT | ft::FT_LOAD_TARGET_NORMAL,
                ) != 0
            {
                return None;
            }
            let slot = &*(*self.face).glyph;
            let advances = (
                slot.advance.x as f32 / 64.0,
                slot.linearHoriAdvance as f32 / 65536.0,
            );
            self.advances.insert(key, advances);
            Some(advances)
        }
    }
}

impl Drop for HintedMetrics {
    fn drop(&mut self) {
        unsafe {
            ft::FT_Done_Face(self.face);
            ft::FT_Done_FreeType(self.library);
        }
    }
}
