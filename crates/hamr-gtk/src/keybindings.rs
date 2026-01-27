//! Keyboard shortcut parsing and matching utilities.

use gtk4::gdk;

/// Parse a shortcut string (e.g., "Ctrl+1", "Ctrl+Shift+A") and check if it matches the key event.
pub fn shortcut_matches(
    shortcut: &str,
    keyval: gdk::Key,
    ctrl: bool,
    shift: bool,
    alt: bool,
) -> bool {
    let parts: Vec<&str> = shortcut.split('+').map(str::trim).collect();
    if parts.is_empty() {
        return false;
    }

    let mut expected_ctrl = false;
    let mut expected_shift = false;
    let mut expected_alt = false;
    let mut key_part = "";

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => expected_ctrl = true,
            "shift" => expected_shift = true,
            "alt" => expected_alt = true,
            _ => key_part = part,
        }
    }

    if ctrl != expected_ctrl || shift != expected_shift || alt != expected_alt {
        return false;
    }

    let Some(expected_key) = parse_key(key_part) else {
        return false;
    };

    keyval == expected_key
}

/// Parse a key string to a GDK key value.
fn parse_key(key: &str) -> Option<gdk::Key> {
    Some(match key.to_lowercase().as_str() {
        "1" => gdk::Key::_1,
        "2" => gdk::Key::_2,
        "3" => gdk::Key::_3,
        "4" => gdk::Key::_4,
        "5" => gdk::Key::_5,
        "6" => gdk::Key::_6,
        "7" => gdk::Key::_7,
        "8" => gdk::Key::_8,
        "9" => gdk::Key::_9,
        "0" => gdk::Key::_0,
        "a" => gdk::Key::a,
        "b" => gdk::Key::b,
        "c" => gdk::Key::c,
        "d" => gdk::Key::d,
        "e" => gdk::Key::e,
        "f" => gdk::Key::f,
        "g" => gdk::Key::g,
        "h" => gdk::Key::h,
        "i" => gdk::Key::i,
        "j" => gdk::Key::j,
        "k" => gdk::Key::k,
        "l" => gdk::Key::l,
        "m" => gdk::Key::m,
        "n" => gdk::Key::n,
        "o" => gdk::Key::o,
        "p" => gdk::Key::p,
        "q" => gdk::Key::q,
        "r" => gdk::Key::r,
        "s" => gdk::Key::s,
        "t" => gdk::Key::t,
        "u" => gdk::Key::u,
        "v" => gdk::Key::v,
        "w" => gdk::Key::w,
        "x" => gdk::Key::x,
        "y" => gdk::Key::y,
        "z" => gdk::Key::z,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_matches_simple_key() {
        assert!(shortcut_matches("a", gdk::Key::a, false, false, false));
        assert!(!shortcut_matches("a", gdk::Key::b, false, false, false));
    }

    #[test]
    fn test_shortcut_matches_with_ctrl() {
        assert!(shortcut_matches("Ctrl+a", gdk::Key::a, true, false, false));
        assert!(!shortcut_matches(
            "Ctrl+a",
            gdk::Key::a,
            false,
            false,
            false
        ));
        assert!(!shortcut_matches("Ctrl+a", gdk::Key::b, true, false, false));
    }

    #[test]
    fn test_shortcut_matches_with_shift() {
        assert!(shortcut_matches("Shift+b", gdk::Key::b, false, true, false));
        assert!(!shortcut_matches(
            "Shift+b",
            gdk::Key::b,
            false,
            false,
            false
        ));
    }

    #[test]
    fn test_shortcut_matches_with_alt() {
        assert!(shortcut_matches("Alt+c", gdk::Key::c, false, false, true));
        assert!(!shortcut_matches("Alt+c", gdk::Key::c, false, false, false));
    }

    #[test]
    fn test_shortcut_matches_combo() {
        assert!(shortcut_matches(
            "Ctrl+Shift+a",
            gdk::Key::a,
            true,
            true,
            false
        ));
        assert!(!shortcut_matches(
            "Ctrl+Shift+a",
            gdk::Key::a,
            true,
            false,
            false
        ));
        assert!(!shortcut_matches(
            "Ctrl+Shift+a",
            gdk::Key::a,
            false,
            true,
            false
        ));
    }

    #[test]
    fn test_shortcut_matches_numbers() {
        assert!(shortcut_matches("Ctrl+1", gdk::Key::_1, true, false, false));
        assert!(shortcut_matches("Ctrl+9", gdk::Key::_9, true, false, false));
        assert!(shortcut_matches("Ctrl+0", gdk::Key::_0, true, false, false));
    }

    #[test]
    fn test_shortcut_matches_case_insensitive() {
        assert!(shortcut_matches("ctrl+A", gdk::Key::a, true, false, false));
        assert!(shortcut_matches("CTRL+a", gdk::Key::a, true, false, false));
        assert!(shortcut_matches(
            "Control+a",
            gdk::Key::a,
            true,
            false,
            false
        ));
    }

    #[test]
    fn test_shortcut_matches_with_spaces() {
        assert!(shortcut_matches(
            "Ctrl + a",
            gdk::Key::a,
            true,
            false,
            false
        ));
        assert!(shortcut_matches(
            "Ctrl + Shift + b",
            gdk::Key::b,
            true,
            true,
            false
        ));
    }

    #[test]
    fn test_shortcut_matches_empty() {
        assert!(!shortcut_matches("", gdk::Key::a, false, false, false));
    }

    #[test]
    fn test_shortcut_matches_unknown_key() {
        assert!(!shortcut_matches("Ctrl+?", gdk::Key::a, true, false, false));
    }
}
