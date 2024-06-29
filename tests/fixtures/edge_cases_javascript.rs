pub const FIXTURE: &[(&str, &str)] = &[
    ("VAR2", ""),
    ("VAR3", "EGG BACON\" \"AND SPAM"),
    ("VAR5", "FOO  BAR"),
    ("VAR6", "FOO  BAR"),
    ("VAR8", "FOO\" "),
    ("BAR1", "BAZ\""),
    ("VAR9", "FOO\nBAR2=BAZ"),
    ("VAR10", "\"FOO  BAR\"  BAZ  BLA"),
    ("VAR12", ""),
    ("VAR13", "TEXT"),
    ("VAR14", "#NO COMMNET"),
    ("VAR15", "#NO COMMNET"),
    ("VAR16", "double quoted backslash:\\\\double quote:\\\"single quote:\\'newline:\ntab:\\tbackspace:\\bformfeed:\\fcarrige return:\runicode ä:\\u00e4"),
    ("VAR17", "single quoted backslash:\\\\double quote:\\\"single quote:\\'newline:\\ntab:\\tbackspace:\\bformfeed:\\fcarrige return:\\runicode ä:\\u00e4"),
    ("VAR18", "no quote backslash:\\\\double quote:\\\"single quote:\\'newline:\\ntab:\\tbackspace:\\bformfeed:\\fcarrige return:\\runicode ä:\\u00e4"),
    ("VAR19", "FOO"),
    ("VAR20", "FOO\\nBAR"),
    ("VAR21", "FOO\nBAR"),
    ("VAR22", "FOO \\"),
    ("VAR23", "double\\\nquoted"),
    ("VAR24", "\"double"),
    ("VAR25", "double\nquoted"),
    ("VAR26", "single'\"\\'\"'quoted"),
    ("VAR27", "single''quoted"),
    ("VAR28", "single-quoted"),
    ("VAR29", "single-quoted"),
    ("VAR30", "single-quoted"),
    ("VAR31", "single-quoted"),
    ("VAR32", "single\nquoted"),
    ("VAR33", "back\nticks"),
    ("VAR34", "\"FOO BAR \"BAZ"),
    ("VAR35", "FOO\" BAR BAZ\""),
    ("VAR36", "\""),
    ("VAR37", "EXPORT!"),
    ("VAR37B", "VAR37B"),
    ("VAR37C", "VAR37C"),
    ("JSON1", "{\"foo\": \"bar \\n no quotes"),
    ("JSON2", "\"{\"foo\": \"bar \n double quotes"),
    ("JSON3", "{\"foo\": \"bar \\n single quotes #\"}"),
    ("JSON4", "{\"foo\": \"bar \\n backticks #\"}"),
    ("PRE_DEFINED", "not override"),
    ("VAR38", "$VAR35"),
    ("VAR39", "X ${VAR35} X $VAR34"),
    ("VAR40", "X${VAR35}X"),
    ("VAR41", "X${VAR35} $ \\$ ${VAR35}X"),
    ("VAR42", "X${VAR35} $ \\$ ${VAR35}X"),
    ("VAR43", "${UNSET:-\n  multiline fallback!\n  variable substitution?\n  VAR5=$VAR5\n  # not a comment?\n}"),
    ("EOF", "\"FOO"),
];
