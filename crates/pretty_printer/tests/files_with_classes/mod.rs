use super::{test, *};

#[test]
fn empty_class() {
	let source = "class Test{}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class Test {
	}
	");
}

#[test]
fn empty_ctor() {
	let source = "class Test{constructor(){}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class Test {
	  constructor() {}
	}
	");
}

#[test]
fn single_method() {
	let source =
		"class Test{constructor(){this.bar=10;}givemebar(){return this.bar;}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class Test {
	  constructor() {
	    this.bar = 10;
	  }
	  givemebar() {
	    return this.bar;
	  }
	}
	");
}

#[test]
fn extending_super_class() {
	let source = "class Foo extends Bar{constructor(name){super(name);}getName(){return super.getName();}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class Foo extends Bar {
	  constructor(name) {
	    super(name);
	  }
	  getName() {
	    return super.getName();
	  }
	}
	");
}

#[test]
fn consecutive_class_decls() {
	let source = "class A{}class B extends A{constructor(){super();}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class A {
	}
	class B extends A {
	  constructor() {
	    super();
	  }
	}
	");
}

#[test]
fn static_methods() {
	let source = "class Employer{static count(){this._counter = (this._counter || 0) + 1; return this._counter;}}";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	class Employer {
	  static count() {
	    this._counter = (this._counter || 0) + 1;
	    return this._counter;
	  }
	}
	");
}

#[test]
fn class_expressions() {
	let source = "new(class{constructor(){debugger}})";
	let out = format2(source).unwrap();
	assert_snapshot!(out, @"
	new (class {
	  constructor() {
	    debugger
	  }
	}
	)
	");
}
