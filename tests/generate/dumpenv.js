const DEFAULT_KEYS = [
    'VAR1',
    'VAR2',
    'VAR3',
    '  VAR3 ',
    'VAR4',
    '"VAR4"',
    '"VAR 4"',
    'VAR 4',
    'VAR5',
    'VAR6',
    'VAR7',
    'VAR8',
    'BAR1',
    'VAR9',
    'BAR2',
    'VAR10',
    'VAR12',
    'VAR13',
    'VAR14',
    'VAR15',
    'VAR16',
    'VAR17',
    'VAR18',
    'VAR19',
    'VAR20',
    'VAR21',
    'VAR22',
    'VAR23',
    'VAR24',
    'VAR25',
    'VAR26',
    'VAR27',
    'VAR28',
    'VAR29',
    'VAR30',
    'VAR31',
    'VAR32',
    'VAR33',
    'VAR34',
    'VAR35',
    'VAR36',
    'VAR37',
    'VAR37B',
    'VAR37C',
    'JSON1',
    'JSON2',
    'JSON3',
    'JSON4',
    'PRE_DEFINED',
    'VAR38',
    'VAR39',
    'VAR40',
    'VAR41',
    'VAR42',
    'VAR43',
    'EOF',
    'FOO',
    'BAR',
    'BAZ',
];

function quote(str) {
    // also patching messed up encoding handling in python-dotenv-cli
    const escaped = str.replaceAll('Ã¤', 'ä').replace(/[\\"\u007F\u0000-\u001F\u00FF-\uD7FF\uE000-\uFFFF]|[\uD800-\uDBFF][\uDC00-\uDFFF]/g, ch => {
        switch (ch) {
            case '\n': return '\\n';
            case '\r': return '\\r';
            case '\t': return '\\t';
            case '"':  return '\\"';
            case '\\': return '\\\\';
            default:
                return `\\u{${ch.codePointAt(0).toString(16)}}`;
        }
    });
    return `"${escaped}"`;
}

function dumpEnv() {
    const keys = process.argv.length >= 3 ? process.argv.slice(2) : DEFAULT_KEYS;
    console.log("pub const FIXTURE: &[(&str, &str)] = &[");
    for (const key of keys) {
        const value = process.env[key];
        if (value !== undefined) {
            console.log(`    (${quote(key)}, ${quote(value)}),`);
        }
    }
    console.log("];");
}

dumpEnv();
