#[macro_export]
macro_rules! assert_env_eq {
    ($env:ident, $fixture:expr) => {
        for (key, expected_value) in $fixture {
            let actual_value = $env.get(OsStr::new(key));

            assert_eq!(actual_value.is_some(), true, "{key} is expected to be set, but isn't");
            let actual_value = actual_value.unwrap();
            assert_eq!(actual_value, expected_value, "{key} is expected to be {expected_value:?}, but is {actual_value:?}");
        }
    };
}
