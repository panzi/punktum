function dumpEnv() {
    const env = {};
    console.log("pub const FIXTURE: &[(&str, &str)] = &[");
    for (const key of [
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
    ]) {
        const value = process.env[key];
        if (value !== undefined) {
            console.log(`    (${JSON.stringify(key)}, ${JSON.stringify(value)}),`);
        }
    }
    console.log("];");
}

dumpEnv();
