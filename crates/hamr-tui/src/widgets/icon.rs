//! Icon name to display string mapping.
//!
//! Maps common icon names to simple ASCII representations for terminal display.

/// Convert an icon name to a display string.
///
/// Returns a simple ASCII representation for known icons,
/// or the original icon name for unknown icons.
///
/// This is the primary function used for inline icon rendering.
#[must_use]
pub fn icon_to_str(icon: &str) -> &str {
    match icon {
        "timer" | "schedule" => "T",
        "pause" => "||",
        "play" | "play_arrow" => ">",
        "stop" => "[]",
        "check" | "done" => "+",
        "close" | "cancel" => "x",
        "warning" => "!",
        "error" => "X",
        "info" => "i",
        "notification" | "notifications" => "*",
        "download" => "v",
        "upload" => "^",
        "refresh" => "@",
        "settings" | "gear" => "#",
        "music_note" => "~",
        "volume_up" => "))",
        "volume_off" => "x)",
        _ => icon, // Return icon name as-is for unknown icons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_icons() {
        assert_eq!(icon_to_str("check"), "+");
        assert_eq!(icon_to_str("play"), ">");
        assert_eq!(icon_to_str("pause"), "||");
        assert_eq!(icon_to_str("warning"), "!");
    }

    #[test]
    fn test_unknown_icon_returns_original() {
        assert_eq!(icon_to_str("unknown_icon"), "unknown_icon");
        assert_eq!(icon_to_str("star"), "star");
    }
}
