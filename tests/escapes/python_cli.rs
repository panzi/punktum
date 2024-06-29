pub const FIXTURE: &[(&str, &str)] = &[
    ("BASIC", "\r,\n,\t,\u{b},\u{c},\u{7},\u{8}"),
    ("BACKSLASH", "\\"),
    ("SINGLE_QUOTED1", "\\'"),
    ("SINGLE_QUOTED2", "\\'"),
    ("OCT2", "++"),
    ("OCT3", "oct"),
    ("HEX", "HEX."),
    ("UTF16", "ä"),
    ("UTF32_8", "\u{1f603}"),
    ("ESCAPED_NEWLINE", "\"\\"),
];
