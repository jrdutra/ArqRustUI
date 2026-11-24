//! Conversions between UTF-8 `str` and Windows UTF-16.

/// Converts UTF-8 to NUL-terminated UTF-16 for Windows.
///
/// Lossless conversion. Interior NUL truncates Windows text; validate with [`has_interior_nul`].
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Whether text has an interior NUL that would truncate Windows strings.
pub fn has_interior_nul(text: &str) -> bool {
    text.contains('\0')
}

/// Converts UTF-16 buffer to String, stopping at first NUL if present.
///
/// Invalid unpaired surrogates become U+FFFD rather than errors.
pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Reads NUL-terminated UTF-16 from pointer.
///
/// Null pointers return empty strings.
///
/// # Safety
/// `ptr` must be null or readable NUL-terminated `u16` data for the entire call.
pub unsafe fn from_wide_ptr(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

/// Copies into fixed UTF-16 buffer, truncating if needed and retaining NUL termination. Never splits surrogate pairs.
pub fn copy_to_buffer(text: &str, buffer: &mut [u16]) {
    if buffer.is_empty() {
        return;
    }
    let capacity = buffer.len() - 1;
    let mut written = 0;
    for ch in text.chars() {
        let mut units = [0u16; 2];
        let encoded = ch.encode_utf16(&mut units);
        if written + encoded.len() > capacity {
            break;
        }
        buffer[written..written + encoded.len()].copy_from_slice(encoded);
        written += encoded.len();
    }
    buffer[written] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_ascii_and_accents() {
        let text = "Olá, Windows! Ação — ñ";
        assert_eq!(from_wide(&to_wide(text)), text);
    }

    #[test]
    fn round_trip_outside_bmp() {
        let text = "emoji 😀 e CJK 漢字 𠮷";
        let wide = to_wide(text);
        assert_eq!(*wide.last().unwrap(), 0);
        assert!(wide.len() > text.chars().count());
        assert_eq!(from_wide(&wide), text);
    }

    #[test]
    fn from_wide_stops_at_nul_and_handles_invalid() {
        assert_eq!(from_wide(&[b'a' as u16, 0, b'b' as u16]), "a");
        assert_eq!(from_wide(&[0xD800]), "\u{FFFD}");
        assert_eq!(from_wide(&[]), "");
    }

    #[test]
    fn pointer_reading() {
        let wide = to_wide("ponteiro");
        // SAFETY: `wide` termina em NUL e vive durante a chamada.
        assert_eq!(unsafe { from_wide_ptr(wide.as_ptr()) }, "ponteiro");
        // SAFETY: ponteiro nulo é aceito explicitamente.
        assert_eq!(unsafe { from_wide_ptr(std::ptr::null()) }, "");
    }

    #[test]
    fn fixed_buffer_truncates_safely() {
        let mut buffer = [0xFFFFu16; 4];
        copy_to_buffer("ab😀", &mut buffer);
        assert_eq!(from_wide(&buffer), "ab");
        copy_to_buffer("abcdef", &mut buffer);
        assert_eq!(from_wide(&buffer), "abc");
        assert!(has_interior_nul("a\0b"));
    }
}
