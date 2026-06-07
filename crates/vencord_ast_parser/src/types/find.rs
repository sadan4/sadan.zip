use oxc::span::Span;

#[derive(Clone, Copy, Debug)]
pub enum FindType {
	Component,
	ByProps,
	Store,
	ByCode,
	ModuleId,
	ComponentByCode,
	CssClasses,
}

pub struct AnyFindType {
	pub find_type: FindType,
	pub lazy: bool,
}

#[expect(
	clippy::to_string_trait_impl,
	reason = "we don't want to provide a display impl"
)]
impl ToString for AnyFindType {
	fn to_string(&self) -> String {
		use FindType as FT;
		const MAX_CAP: usize = 23;
		let mut ret = String::with_capacity(MAX_CAP);
		ret.push_str("find");
		let next = match self.find_type {
			FT::Component => "Component",
			FT::ByProps => "ByProps",
			FT::Store => "Store",
			FT::ByCode => "ByCode",
			FT::ModuleId => "ModuleId",
			FT::ComponentByCode => "ComponentByCode",
			FT::CssClasses => "CssClasses",
		};
		ret.push_str(next);
		if self.lazy {
			ret.push_str("Lazy");
		}
		debug_assert!(
			ret.len() <= MAX_CAP,
			"increase max cap to {}",
			ret.len()
		);
		ret
	}
}

/// was `FindNode` in js
pub enum FindArg {
	String(String),
	Regex { pattern: String, flags: String },
	Function(String),
}

pub struct FindData {
	pub kind: AnyFindType,
	pub args: Vec<FindArg>,
}

pub struct FindUse {
	pub range: Span,
	pub data: FindData,
}
