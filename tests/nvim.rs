use nvimkata::nvim::{escape_for_lua_sq, normalize};

#[test]
fn test_normalize_trims_trailing_whitespace() {
    assert_eq!(normalize("hello   \nworld  \n"), "hello\nworld");
}

#[test]
fn test_normalize_matching() {
    let a = "line1\nline2\n";
    let b = "line1\nline2";
    assert_eq!(normalize(a), normalize(b));
}

#[test]
fn test_normalize_different() {
    let a = "hello";
    let b = "world";
    assert_ne!(normalize(a), normalize(b));
}

#[test]
fn test_escape_for_lua_sq() {
    assert_eq!(escape_for_lua_sq("hello"), "hello");
    assert_eq!(escape_for_lua_sq("it's"), "it\\'s");
    assert_eq!(escape_for_lua_sq("a\\b"), "a\\\\b");
    assert_eq!(escape_for_lua_sq("line1\nline2"), "line1\\nline2");
    assert_eq!(escape_for_lua_sq("cr\rhere"), "cr\\rhere");
}
