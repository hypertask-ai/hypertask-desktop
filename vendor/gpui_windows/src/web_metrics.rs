use collections::HashMap;
use freetype_sys as ft;
use std::ptr;
use windows::Win32::Graphics::DirectWrite::{IDWriteFontFace3, IDWriteFontFileStream};

pub(super) struct WebMetrics {
    library: ft::FT_Library,
    face: ft::FT_Face,
    stream: IDWriteFontFileStream,
    fragment: *mut std::ffi::c_void,
    advances: HashMap<(u32, u16), (f32, f32)>,
}

// FreeType and the retained DirectWrite fragment stay under the text-system lock.
unsafe impl Send for WebMetrics {}
unsafe impl Sync for WebMetrics {}

impl WebMetrics {
    pub(super) fn new(font: &IDWriteFontFace3) -> Option<Self> {
        unsafe {
            let mut count = 1;
            let mut file = None;
            font.GetFiles(&mut count, Some(&mut file)).ok()?;
            let file = file?;
            let mut key = ptr::null_mut();
            let mut key_len = 0;
            file.GetReferenceKey(&mut key, &mut key_len).ok()?;
            let stream = file.GetLoader().ok()?.CreateStreamFromKey(key, key_len).ok()?;
            let length = stream.GetFileSize().ok()?;
            let mut data = ptr::null_mut();
            let mut fragment = ptr::null_mut();
            stream.ReadFileFragment(&mut data, 0, length, &mut fragment).ok()?;
            let mut library = ptr::null_mut();
            let mut face = ptr::null_mut();
            if ft::FT_Init_FreeType(&mut library) != 0 {
                stream.ReleaseFileFragment(fragment);
                return None;
            }
            let Ok(length) = length.try_into() else {
                ft::FT_Done_FreeType(library);
                stream.ReleaseFileFragment(fragment);
                return None;
            };
            if ft::FT_New_Memory_Face(library, data.cast(), length, font.GetIndex() as ft::FT_Long, &mut face) != 0 {
                ft::FT_Done_FreeType(library);
                stream.ReleaseFileFragment(fragment);
                return None;
            }
            Some(Self { library, face, stream, fragment, advances: HashMap::default() })
        }
    }

    pub(super) fn advance(&mut self, size: f32, glyph: u16) -> Option<(f32, f32)> {
        if !size.is_finite() || size <= 0. || !ft::FT_IS_SCALABLE(self.face) || ft::FT_IS_FIXED_WIDTH(self.face) {
            return None;
        }
        let key = (size.to_bits(), glyph);
        if let Some(value) = self.advances.get(&key) {
            return Some(*value);
        }
        unsafe {
            if ft::FT_Set_Char_Size(self.face, 0, (size * 64.).round() as ft::FT_F26Dot6, 72, 72) != 0
                || ft::FT_Load_Glyph(self.face, glyph.into(), ft::FT_LOAD_DEFAULT | ft::FT_LOAD_TARGET_NORMAL) != 0 {
                return None;
            }
            let slot = &*(*self.face).glyph;
            let value = (slot.advance.x as f32 / 64., slot.linearHoriAdvance as f32 / 65536.);
            self.advances.insert(key, value);
            Some(value)
        }
    }
}

impl Drop for WebMetrics {
    fn drop(&mut self) {
        unsafe {
            ft::FT_Done_Face(self.face);
            ft::FT_Done_FreeType(self.library);
            self.stream.ReleaseFileFragment(self.fragment);
        }
    }
}
