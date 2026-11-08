pub fn platform_name() -> &'static str {
    #[cfg(windows)]
    {
        "Windows"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        "Unsupported"
    }
}

pub fn known_limitations() -> &'static [&'static str] {
    &[
        "Xbox / Microsoft Store games: save locations are sandboxed and not reliably discoverable from metadata; register them manually with a custom save path.",
        "EA App and Ubisoft Connect: no public local manifest format is parsed in this release; games from those launchers can be added manually.",
        "Cloud-only saves (Steam Cloud with no local files) have nothing local to back up until the game writes files to disk.",
    ]
}
