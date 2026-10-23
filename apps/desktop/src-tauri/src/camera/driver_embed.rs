//! Pure rules shared by `build.rs` and the app (ADR-0026): whether a file is a real
//! driver worth embedding, and how to read `MB_CAMERA_DRIVER_VERSION` from the native
//! header. `build.rs` includes this file to decide what to embed; `driver.rs` includes
//! it in tests, so the rules are covered without running the build script.

/// Anything at or below this size is the committed placeholder, not a driver.
pub const MIN_DRIVER_BYTES: usize = 1024;

/// A Windows PE image (`MZ`) that is not the placeholder and was not truncated.
pub fn is_real_dll(bytes: &[u8]) -> bool {
    bytes.len() > MIN_DRIVER_BYTES && bytes.starts_with(b"MZ")
}

/// Reads `#define MB_CAMERA_DRIVER_VERSION <n>u` from `meetburner_camera.h`.
pub fn parse_driver_version(header: &str) -> Option<u32> {
    header.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        if words.next() != Some("#define") || words.next() != Some("MB_CAMERA_DRIVER_VERSION") {
            return None;
        }
        words.next()?.trim_end_matches(['u', 'U']).parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_at_least(size: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; size];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes
    }

    #[test]
    fn the_placeholder_is_not_a_driver() {
        assert!(!is_real_dll(b""));
        assert!(!is_real_dll(b"MZ"), "MZ alone is too small");
        assert!(
            !is_real_dll(&image_at_least(MIN_DRIVER_BYTES)),
            "1 KB or less is the placeholder"
        );
        assert!(
            !is_real_dll(&vec![b'?'; MIN_DRIVER_BYTES + 1]),
            "must start with MZ"
        );
        assert!(is_real_dll(&image_at_least(MIN_DRIVER_BYTES + 1)));
    }

    #[test]
    fn reads_the_driver_version_define() {
        assert_eq!(
            parse_driver_version("#pragma once\n#define MB_CAMERA_DRIVER_VERSION 2u\n"),
            Some(2)
        );
        assert_eq!(
            parse_driver_version("#define MB_CAMERA_DRIVER_VERSION 7\n"),
            Some(7)
        );
        assert_eq!(
            parse_driver_version("  #define\tMB_CAMERA_DRIVER_VERSION\t12U  \n"),
            Some(12)
        );
    }

    #[test]
    fn a_missing_version_define_is_unknown() {
        assert_eq!(parse_driver_version("#define MB_CAMERA_VERSION 1u\n"), None);
        assert_eq!(
            parse_driver_version("// #define MB_CAMERA_DRIVER_VERSION 2u\n"),
            None
        );
        assert_eq!(parse_driver_version(""), None);
    }
}
