pub const FIXTURE: &[(&str, &str)] = &[
    ("VAR1", "{{\n}"),
    ("VAR2", "{baz\nbla}"),
    ("VAR3", "$UNSET:-TEXT"),
    ("VAR4", "$VAR3:-TEXT"),
    ("VAR5", "$VAR3$VAR4"),
    ("VAR6", "\nVAR4=$VAR4\nVAR5=$VAR5\n"),
    ("VAR7", "$VAR3$VAR4"),
];
