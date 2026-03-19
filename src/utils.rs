use std::ffi::c_char;

/// Converts a C string to an owned Rust String, with a length limit (the size of `buffer`).
pub fn c_buffer_to_string(buffer: &[c_char]) -> String {
    // cap the length to the size of the buffer
    let length = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let chars = &buffer[..length];
    // convert to bytes
    // SAFETY: `c_char` is either `i8` or `u8`, and a slice of that can be converted safely to a slice of `u8`s
    let bytes = unsafe { &*(chars as *const [c_char] as *const [u8]) };
    // convert to utf-8
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use std::ffi::c_char;

    use super::c_buffer_to_string;

    fn c_array(bytes: &[u8]) -> &[c_char] {
        unsafe { &*(bytes as *const [u8] as *const [c_char]) }
    }

    #[test]
    fn valid_c_buffer_to_string() {
        let invalid = b"Hello \xF0\x90\x80World\0";
        assert_eq!(c_buffer_to_string(c_array(b"abc\0")), "abc"); // null-terminated
        assert_eq!(c_buffer_to_string(c_array(b"abc")), "abc"); // NOT null-terminated
        assert_eq!(c_buffer_to_string(c_array(invalid)), "Hello �World"); // invalid utf-8 chars
        assert_eq!(c_buffer_to_string(c_array(b"\0\0\0\0")), ""); // multiple nulls
    }
}
