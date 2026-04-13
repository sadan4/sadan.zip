use super::{*, test};

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
