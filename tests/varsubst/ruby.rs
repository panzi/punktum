pub const FIXTURE: &[(&str, &str)] = &[
    ("VAR1", ":-{{}\n}"),
    ("VAR2", ":-{baz}\nbla}"),
    ("VAR3", ":-TEXT"),
    ("VAR4", ":-TEXT:-TEXT"),
    ("VAR5", ":-TEXT:-TEXT:-TEXT"),
    ("VAR6", "\nVAR4=:-TEXT:-TEXT\nVAR5=:-TEXT:-TEXT:-TEXT\n"),
    ("VAR7", "$VAR3$VAR4"),
];
