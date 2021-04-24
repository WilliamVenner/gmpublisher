const BACKSLASH_BYTE: u8 = '\\' as u8;
const SINGLE_QUOTE_BYTE: u8 = '\'' as u8;
pub fn escape_single_quoted_json<S: Into<String>>(str: S) -> String {
	let mut str = str.into();
	let bytes: &mut Vec<u8> = unsafe { str.as_mut_vec() };
	let mut i = 0;
	while i < bytes.len() {
		let byte = bytes[i];
		match byte {
			BACKSLASH_BYTE | SINGLE_QUOTE_BYTE => {
				bytes.insert(i, BACKSLASH_BYTE);
				i += 1;
			}
			_ => {}
		}
		i += 1;
	}
	debug_assert!(String::from_utf8(bytes.to_vec()).is_ok());
	str
}

#[test]
fn test_escape_single_quoted_json() {
	let dangerous_json = String::from(
		r#"{"test":"don\\🚀🐱‍👤\\'t forget to escape me!🚀🐱‍👤","te🚀🐱‍👤st2":"don't forget to escape me!","test3":"\\🚀🐱‍👤\\\\'''\\\\🚀🐱‍👤\\\\🚀🐱‍👤\\'''''"}"#,
	);

	let definitely_escaped_dangerous_json = dangerous_json.clone().replace('\\', "\\\\").replace('\'', "\\'");
	let escape_single_quoted_json_test = escape_single_quoted_json(dangerous_json);

	let result = r#"{"test":"don\\\\🚀🐱‍👤\\\\\'t forget to escape me!🚀🐱‍👤","te🚀🐱‍👤st2":"don\'t forget to escape me!","test3":"\\\\🚀🐱‍👤\\\\\\\\\'\'\'\\\\\\\\🚀🐱‍👤\\\\\\\\🚀🐱‍👤\\\\\'\'\'\'\'"}"#;
	assert_eq!(definitely_escaped_dangerous_json, result);
	assert_eq!(escape_single_quoted_json_test, result);
}
