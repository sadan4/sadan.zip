use super::{test, *};

#[test]
fn file_with_one_leading_comment() {
	let source = "// This is a starting comment
console.log('5');";
	let out = format2(source).unwrap();
	assert_eq!(out, source);
}

#[test]
fn hashbangs() {
	let source = "#! hashbang
{{{console.log(1)}}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	#! hashbang
	{
	  {
	    {
	      console.log(1)
	    }
	  }
	}
	");
}

#[test]
fn one_trailing_comment() {
	let source = "console.log('5'); // This is a trailing comment";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	console.log('5');
	// This is a trailing comment
	");
}

#[test]
fn one_trailing_comment_on_new_line() {
	let source = "console.log('5');
// This is a new line comment";
	let out = format2(source).unwrap();
	assert_eq!(out, source);
}

#[test]
fn two_leading_comments() {
	let source = "// This is a starting line comment
/* This is a starting block comment */
console.log('5');";
	let out = format2(source).unwrap();
	assert_eq!(out, source);
}

#[test]
fn two_trailing_comments() {
	let source = "console.log('5'); // This is a trailing comment same line
// This is a trailing new line comment";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	console.log('5');
	// This is a trailing comment same line
	// This is a trailing new line comment
	");
}

#[test]
fn two_leading_and_trailing_comments() {
	let source = "// This is a starting line comment
/* This is a starting block comment */
console.log('5'); // This is a trailing comment same line
// This is a trailing new line comment";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	// This is a starting line comment
	/* This is a starting block comment */
	console.log('5');
	// This is a trailing comment same line
	// This is a trailing new line comment
	");
}

#[test]
fn hashbang_leading_and_trailing() {
	let source = "#! hashbang
// This is a starting line comment
/* This is a starting block comment */
console.log('5'); // This is a trailing comment same line
// This is a trailing new line comment";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	#! hashbang
	// This is a starting line comment
	/* This is a starting block comment */
	console.log('5');
	// This is a trailing comment same line
	// This is a trailing new line comment
	");
}
