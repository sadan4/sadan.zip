use std::{
	borrow::Cow,
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

use ast_parser::{AstParser as _, exts::MemberExprRef};
use explorer_types::{ModuleId, SpannedId};
use num_bigint::BigInt;
use ordered_float::NotNan;
use oxc::{
	allocator::GetAddress,
	ast::{
		AstKind,
		ast::{
			Argument,
			ArrayExpressionElement,
			AssignmentOperator,
			CallExpression,
			ChainElement,
			ChainExpression,
			Expression,
			IdentifierReference,
			LogicalOperator,
			NewExpression,
			ObjectExpression,
			ObjectPropertyKind,
			PropertyKey,
			PropertyKind,
			UnaryOperator,
		},
		match_expression,
	},
	semantic::SymbolId,
	span::{GetSpan, Span},
};
use parser_diag::{PResult, ParserDiagnostic};
use tracing::error;

use crate::{
	WebpackAstParser,
	parser::export_map::{ExportMapKey, ExportValue},
};
use member::Property;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantPropertyKey {
	String(String),
	Number(NotNan<f64>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantValue<T> {
	Number(NotNan<f64>),
	NaN,
	BigInt(num_bigint::BigInt),
	Boolean(bool),
	String(String),
	Array(Vec<T>),
	Object(BTreeMap<ConstantPropertyKey, T>),
	Set(BTreeSet<T>),
	Undefined,
	Null,
}

impl<T> ConstantValue<T> {
	fn is_falsy(&self) -> bool {
		match self {
			Self::Number(n) => *n == 0.,
			Self::BigInt(n) => *n == BigInt::ZERO,
			Self::Boolean(b) => !*b,
			Self::String(s) => s.is_empty(),
			Self::NaN | Self::Undefined | Self::Null => true,
			Self::Array(_) | Self::Object(_) | Self::Set(_) => false,
		}
	}
	fn is_truthy(&self) -> bool {
		!self.is_falsy()
	}
	const fn is_nullish(&self) -> bool {
		matches!(self, Self::Undefined | Self::Null)
	}
}
pub type ConstantSpannedValue = ConstantValue<SpannedValue>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpannedValue {
	pub value: ConstantSpannedValue,
	/// The source location of this constant value,
	pub span: Span,
	/// The source module id of this value, so [`Self::span`] can be properly resolved
	pub module_id: ModuleId,
}

impl GetSpan for SpannedValue {
	fn span(&self) -> Span {
		self.span
	}
}

impl<T> From<f64> for ConstantValue<T> {
	fn from(value: f64) -> Self {
		if let Ok(value) = NotNan::new(value) {
			Self::Number(value)
		} else {
			debug_assert!(value.is_nan());
			Self::NaN
		}
	}
}

/// The parts of a member expression needed after its key is resolved
struct MemberInfo {
	/// The whole member expression
	span: Span,
	/// `b` in `a.b` and `a[b]`
	key_span: Span,
	/// `a?.b`
	optional: bool,
}

impl MemberInfo {
	fn new(member: MemberExprRef<'_>) -> Self {
		let (span, key_span, optional) = match member {
			MemberExprRef::Static(m) => (m.span, m.property.span, m.optional),
			MemberExprRef::Computed(m) => {
				(m.span, m.expression.span(), m.optional)
			}
			MemberExprRef::Private(m) => (m.span, m.field.span, m.optional),
		};
		Self {
			span,
			key_span,
			optional,
		}
	}
}

/// Something being resolved, to catch cycles like `let x; x = x + 1;`
#[derive(Debug, PartialEq, Eq)]
enum Resolving {
	Symbol(ModuleId, SymbolId),
	Export(ModuleId, Vec<ExportMapKey>),
}

impl<'ast> WebpackAstParser<'ast> {
	/// <https://262.ecma-international.org/#sec-tostring>
	fn to_string<'a>(&self, v: &'a SpannedValue) -> Cow<'a, str> {
		let argument = &v.value;
		// 1. If argument is a String, return argument.
		if let ConstantValue::String(s) = &argument {
			return Cow::Borrowed(s.as_str());
		}
		// 2. If argument is a Symbol, throw a TypeError exception.
		// we don't support symbols
		// 3. If argument is undefined, return "undefined".
		if matches!(&argument, ConstantValue::Undefined) {
			return Cow::Borrowed("undefined");
		}
		// 4. If argument is null, return "null".
		if matches!(&argument, ConstantValue::Null) {
			return Cow::Borrowed("null");
		}
		// 5. If argument is true, return "true".
		if matches!(&argument, ConstantValue::Boolean(true)) {
			return Cow::Borrowed("true");
		}
		// 6. If argument is false, return "false".
		if matches!(&argument, ConstantValue::Boolean(false)) {
			return Cow::Borrowed("false");
		}
		// 7. If argument is a Number, return Number::toString(argument, 10).
		if let ConstantValue::Number(n) = &argument {
			return Cow::Owned(n.to_string());
		}
		// we handle NaN separately
		if matches!(&argument, ConstantValue::NaN) {
			return Cow::Borrowed("NaN");
		}
		// 8. If argument is a BigInt, return BigInt::toString(argument, 10).
		if let ConstantValue::BigInt(n) = &argument {
			return Cow::Owned(n.to_string());
		}
		match argument {
			ConstantValue::Array(_) => Cow::Owned(
				self.array_prototype_join(v, None)
					.unwrap(),
			),
			ConstantValue::Object(_) => Cow::Borrowed("[object Object]"),
			ConstantValue::Set(_) => Cow::Borrowed("[object Set]"),
			// 9. Assert: argument is an Object.
			_ => unreachable!(),
		}
	}
	/// <https://262.ecma-international.org/#sec-array.prototype.join>
	fn array_prototype_join(
		&self,
		arr: &SpannedValue,
		separator: Option<&str>,
	) -> PResult<String> {
		let ConstantValue::Array(elts) = &arr.value else {
			return Err(self.local_err(
				arr,
				"array.prototype.join called on non-array value",
			));
		};
		let mut ret = String::new();
		let sep = separator.unwrap_or(",");
		for (i, elt) in elts.iter().enumerate() {
			if i != 0 {
				ret.push_str(sep);
			}
			let str = self.to_string(elt);
			ret.push_str(&str);
		}
		Ok(ret)
	}
	#[expect(clippy::future_not_send)]
	pub(super) async fn remote_err(
		&self,
		span: &impl GetSpan,
		m_id: ModuleId,
		msg: impl Into<Cow<'static, str>>,
	) -> ParserDiagnostic {
		let ret = try {
			if m_id == self.get_module_id()?.id {
				return self.local_err(span, msg);
			}
			let parser = self
				.try_get_module_parser(SpannedId::unspanned(m_id))
				.await
				.map_err(|e| {
					parser_diag::err_ns("failed to get module parser").s(e)
				})?;
			let source = Arc::clone(parser.get_source());
			let mut err = parser_diag::err(span, msg);
			err.txt = Some(Arc::new(source));
			err
		};
		match ret {
			Ok(d) => d,
			Err(e) => {
				error!("Failed to create remote error: {e:?}");
				e
			}
		}
	}
	fn local_err(
		&self,
		span: &impl GetSpan,
		msg: impl Into<Cow<'static, str>>,
	) -> ParserDiagnostic {
		let source = self.get_arc_source();
		let mut err = parser_diag::err(span, msg);
		err.txt = Some(Arc::new(source));
		err
	}
	/// Resolves the only value `ident` is ever set to
	#[expect(clippy::future_not_send)]
	async fn resolve_ident(
		&self,
		ident: &'ast IdentifierReference<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		let Some(sym) = self
			.sema
			.scoping()
			.get_reference(ident.reference_id())
			.symbol_id()
		else {
			let value = match ident.name.as_str() {
				"undefined" => ConstantValue::Undefined,
				"NaN" => ConstantValue::NaN,
				"Infinity" => ConstantValue::from(f64::INFINITY),
				_ => {
					return Err(
						self.local_err(ident, "unresolved global identifier")
					);
				}
			};
			return Ok(SpannedValue {
				value,
				span: ident.span,
				module_id: self.get_module_id()?.id,
			});
		};
		self.resolve_symbol(sym, ident, resolving)
			.await
	}
	/// Resolves the only value `sym` is ever set to, `at` is where it is used
	#[expect(clippy::future_not_send)]
	async fn resolve_symbol(
		&self,
		sym: SymbolId,
		at: &impl GetSpan,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		let decl_err = || {
			self.local_err(
				&self.sema.scoping().symbol_span(sym),
				"declared here",
			)
		};
		let entry = Resolving::Symbol(self.get_module_id()?.id, sym);
		if resolving.contains(&entry) {
			return Err(self
				.local_err(at, "identifier is defined in terms of itself")
				.s(decl_err()));
		}
		let Some(init) = self.constant_value_of(&sym) else {
			return Err(self
				.local_err(at, "identifier does not have a single value")
				.s(decl_err()));
		};
		resolving.push(entry);
		let ret = Box::pin(self.resolve_value(init, resolving)).await;
		resolving.pop();
		ret
	}
	/// Whether `ident` is the global `name`, and not a local shadowing it
	fn is_global(&self, ident: &IdentifierReference<'_>, name: &str) -> bool {
		ident.name == name
			&& self
				.sema
				.scoping()
				.get_reference(ident.reference_id())
				.symbol_id()
				.is_none()
	}
	/// Resolves the only argument of `call`, `what` names the call in errors
	#[expect(clippy::future_not_send)]
	async fn resolve_only_arg(
		&self,
		call: &impl GetSpan,
		args: &'ast [Argument<'ast>],
		what: &str,
		resolving: &mut Vec<Resolving>,
	) -> PResult<Option<SpannedValue>> {
		match args {
			[] => Ok(None),
			[Argument::SpreadElement(spread)] => Err(self.local_err(
				&**spread,
				format!("spread arguments to {what} are not supported"),
			)),
			[arg] => {
				let arg = arg.to_expression();
				Ok(Some(Box::pin(self.resolve_value(arg, resolving)).await?))
			}
			_ => Err(self.local_err(
				call,
				format!("{what} with more than one argument is not supported"),
			)),
		}
	}
	/// Resolves calls with constant results, only `Object.freeze(value)`
	#[expect(clippy::future_not_send)]
	async fn resolve_call(
		&self,
		call: &'ast CallExpression<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		// `Object.freeze` is never nullish, so `Object?.freeze?.(a)` is the
		// same as `Object.freeze(a)`
		let is_object_freeze = matches!(
			&call.callee,
			Expression::StaticMemberExpression(callee)
				if callee.property.name == "freeze"
					&& matches!(
						&callee.object,
						Expression::Identifier(object)
							if self.is_global(object, "Object")
					)
		);
		if !is_object_freeze {
			return Err(self.local_err(
				call,
				"only calls to `Object.freeze` are supported",
			));
		}
		// freezing doesn't change the value, and non-objects are returned
		// as is
		let value = self
			.resolve_only_arg(call, &call.arguments, "Object.freeze", resolving)
			.await?
			.map_or(ConstantValue::Undefined, |arg| arg.value);
		Ok(SpannedValue {
			value,
			span: call.span,
			module_id: self.get_module_id()?.id,
		})
	}
	/// Resolves `new Set(iterable)`
	#[expect(clippy::future_not_send)]
	async fn resolve_new(
		&self,
		new: &'ast NewExpression<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		let is_set = matches!(
			&new.callee,
			Expression::Identifier(callee) if self.is_global(callee, "Set")
		);
		if !is_set {
			return Err(self.local_err(new, "only `new Set(...)` is supported"));
		}
		let module_id = self.get_module_id()?.id;
		let arg = self
			.resolve_only_arg(new, &new.arguments, "new Set", resolving)
			.await?;
		let elts = match arg {
			None
			| Some(SpannedValue {
				value: ConstantValue::Undefined | ConstantValue::Null,
				..
			}) => Vec::new(),
			Some(SpannedValue {
				value: ConstantValue::Array(elts),
				..
			}) => elts,
			Some(SpannedValue {
				value: ConstantValue::Set(elts),
				..
			}) => elts.into_iter().collect(),
			// strings are iterated by code point
			Some(SpannedValue {
				value: ConstantValue::String(s),
				span,
				module_id,
			}) => s
				.chars()
				.map(|c| SpannedValue {
					value: ConstantValue::String(c.into()),
					span,
					module_id,
				})
				.collect(),
			Some(arg) => {
				return Err(self
					.local_err(new, "the argument to `new Set` is not iterable")
					.s(self
						.remote_err(
							&arg.span,
							arg.module_id,
							"argument defined here",
						)
						.await));
			}
		};
		let mut set = BTreeSet::new();
		for elt in elts {
			// primitives are the same element when they are the same value.
			// objects are the same when they are the same object, which is
			// approximated by being defined at the same place
			let is_object = matches!(
				elt.value,
				ConstantValue::Array(_)
					| ConstantValue::Object(_)
					| ConstantValue::Set(_)
			);
			if is_object
				|| !set
					.iter()
					.any(|e: &SpannedValue| e.value == elt.value)
			{
				set.insert(elt);
			}
		}
		Ok(SpannedValue {
			value: ConstantValue::Set(set),
			span: new.span,
			module_id,
		})
	}
	/// Resolves `{ a: 1, [b]: 2, ...c }`
	#[expect(clippy::future_not_send)]
	async fn resolve_object(
		&self,
		obj: &'ast ObjectExpression<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		let module_id = self.get_module_id()?.id;
		let mut props = BTreeMap::new();
		for prop in &obj.properties {
			match prop {
				ObjectPropertyKind::ObjectProperty(prop) => {
					if prop.kind != PropertyKind::Init {
						return Err(self.local_err(
							&**prop,
							"getters and setters are not constant values",
						));
					}
					let key = match &prop.key {
						PropertyKey::StaticIdentifier(ident) => {
							ConstantPropertyKey::from_string(
								ident.name.as_str(),
							)
						}
						PropertyKey::PrivateIdentifier(ident) => {
							return Err(self.local_err(
								&**ident,
								"private identifiers are not valid object keys",
							));
						}
						key => {
							let key = key.as_expression().unwrap();
							let key =
								Box::pin(self.resolve_value(key, resolving))
									.await?;
							self.property_key(&key)
						}
					};
					// `{ __proto__: a }` sets the prototype, it does not
					// define a property
					if !prop.computed
						&& !prop.shorthand
						&& key
							== ConstantPropertyKey::String(
								"__proto__".to_owned(),
							) {
						return Err(self.local_err(
							&prop.key,
							"setting the prototype of an object is not \
							 supported",
						));
					}
					let value =
						Box::pin(self.resolve_value(&prop.value, resolving))
							.await?;
					props.insert(key, value);
				}
				ObjectPropertyKind::SpreadProperty(spread) => {
					let value = Box::pin(
						self.resolve_value(&spread.argument, resolving),
					)
					.await?;
					let index = |i: usize| {
						#[expect(
							clippy::cast_precision_loss,
							reason = "lengths are small"
						)]
						ConstantPropertyKey::from_number(i as f64)
					};
					match value.value {
						ConstantValue::Object(obj) => props.extend(obj),
						ConstantValue::Array(elts) => props.extend(
							elts.into_iter()
								.enumerate()
								.map(|(i, v)| (index(i), v)),
						),
						ConstantValue::String(s) => {
							// JS spreads strings by UTF-16 code unit, which
							// can't hold half of a surrogate pair
							if s.chars().any(|c| c.len_utf16() != 1) {
								return Err(self.local_err(
									&**spread,
									"spreading strings with characters \
									 outside the BMP is not supported",
								));
							}
							props.extend(s.chars().enumerate().map(
								|(i, c)| {
									(
										index(i),
										SpannedValue {
											value: ConstantValue::String(
												c.into(),
											),
											span: spread.span,
											module_id,
										},
									)
								},
							));
						}
						// no own enumerable properties
						ConstantValue::Number(_)
						| ConstantValue::NaN
						| ConstantValue::BigInt(_)
						| ConstantValue::Boolean(_)
						| ConstantValue::Set(_)
						| ConstantValue::Undefined
						| ConstantValue::Null => {}
					}
				}
			}
		}
		Ok(SpannedValue {
			value: ConstantValue::Object(props),
			span: obj.span,
			module_id,
		})
	}
	/// <https://tc39.es/ecma262/#sec-topropertykey>
	fn property_key(&self, key: &SpannedValue) -> ConstantPropertyKey {
		match &key.value {
			ConstantValue::Number(n) => ConstantPropertyKey::from_number(**n),
			ConstantValue::NaN => ConstantPropertyKey::from_number(f64::NAN),
			_ => ConstantPropertyKey::from_string(&self.to_string(key)),
		}
	}
	/// Resolves `a.b[c]`, either as a property of a constant value or as an
	/// export of another module
	///
	/// Optional accesses in the chain short-circuit the whole chain, as they
	/// do in a [`ChainExpression`]
	#[expect(clippy::future_not_send)]
	async fn resolve_member(
		&self,
		member: MemberExprRef<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		// innermost first, so `a.b[c]` is `[a.b, a.b[c]]`
		let mut members = vec![member];
		loop {
			let next = match members.last().unwrap().left() {
				Expression::StaticMemberExpression(m) => {
					MemberExprRef::Static(m)
				}
				Expression::ComputedMemberExpression(m) => {
					MemberExprRef::Computed(m)
				}
				_ => break,
			};
			members.push(next);
		}
		members.reverse();
		let base = members[0].left();
		let (mut value, start, keys) = if let Some(module_id) =
			self.module_ref_of(base)
		{
			// modules are never nullish, so every key is evaluated
			let mut keys = Vec::with_capacity(members.len());
			for &member in &members {
				keys.push(
					self.member_key(member, resolving)
						.await?,
				);
			}
			let (value, used) = self
				.resolve_export(module_id, &members, &keys, resolving)
				.await?;
			(value, used, keys)
		} else {
			let value = Box::pin(self.resolve_value(base, resolving)).await?;
			(value, 0, Vec::new())
		};
		for (i, &member) in members.iter().enumerate().skip(start) {
			let info = MemberInfo::new(member);
			if info.optional && value.value.is_nullish() {
				// the rest of the chain, including its keys, is skipped
				return Ok(SpannedValue {
					value: ConstantValue::Undefined,
					span: MemberInfo::new(member).span,
					module_id: self.get_module_id()?.id,
				});
			}
			let key = match keys.get(i) {
				Some(key) => key.clone(),
				None => {
					self.member_key(member, resolving)
						.await?
				}
			};
			value = self
				.read_property(value, &info, &key)
				.await?;
		}
		Ok(value)
	}
	/// Resolves `a?.b`, `a?.[b]` and `a?.()`
	#[expect(clippy::future_not_send)]
	async fn resolve_chain(
		&self,
		chain: &'ast ChainExpression<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		match &chain.expression {
			ChainElement::StaticMemberExpression(m) => {
				self.resolve_member(MemberExprRef::Static(m), resolving)
					.await
			}
			ChainElement::ComputedMemberExpression(m) => {
				self.resolve_member(MemberExprRef::Computed(m), resolving)
					.await
			}
			ChainElement::CallExpression(call) => {
				self.resolve_call(call, resolving).await
			}
			_ => Err(self.local_err(
				chain,
				"unsupported optional chain for constant value",
			)),
		}
	}
	/// The key `member` accesses, `b` in `a.b` and `a[b]`
	#[expect(clippy::future_not_send)]
	async fn member_key(
		&self,
		member: MemberExprRef<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<ConstantPropertyKey> {
		match member {
			MemberExprRef::Static(m) => {
				Ok(ConstantPropertyKey::from_string(m.property.name.as_str()))
			}
			MemberExprRef::Computed(m) => {
				let key =
					Box::pin(self.resolve_value(&m.expression, resolving))
						.await?;
				Ok(self.property_key(&key))
			}
			MemberExprRef::Private(m) => Err(self.local_err(
				&m.field,
				"private fields are not supported for constant values",
			)),
		}
	}
	/// Reads `key` of `obj`, the value of the object of the member expression
	/// described by `info`
	async fn read_property(
		&self,
		obj: SpannedValue,
		info: &MemberInfo,
		key: &ConstantPropertyKey,
	) -> PResult<SpannedValue> {
		match member::get_property(&obj.value, key) {
			Ok(Property::Existing(v)) => Ok(v.clone()),
			Ok(Property::New(value)) => Ok(SpannedValue {
				value,
				span: info.span,
				module_id: self.get_module_id()?.id,
			}),
			Err(msg) => Err(self.local_err(&info.span, msg).s(self
				.remote_err(&obj.span, obj.module_id, "object defined here")
				.await)),
		}
	}
	/// Resolves the export of `module_id` named by `keys`, the keys of
	/// `members`, the innermost member accessing the module itself
	///
	/// Returns the value and how many of `members` were used to name the
	/// export, the rest are properties of the value
	#[expect(clippy::future_not_send)]
	async fn resolve_export(
		&self,
		module_id: SpannedId,
		members: &[MemberExprRef<'ast>],
		keys: &[ConstantPropertyKey],
		resolving: &mut Vec<Resolving>,
	) -> PResult<(SpannedValue, usize)> {
		let keys = keys
			.iter()
			.map(|key| ExportMapKey::from(&key.to_string()))
			.collect::<Vec<_>>();
		let remote = self
			.try_get_module_parser(module_id)
			.await?;
		let p = remote.parser();
		let mut map = p.get_export_map_raw();
		let mut used = 0;
		let node = loop {
			let Some(key) = keys.get(used) else {
				// every key named a nested object, use the object itself,
				// preferring the node it was made from
				let node = map.node.or_else(|| {
					map.cjs_default
						.as_deref()
						.and_then(|d| d.try_unwrap_range_ref().ok())?
						.last()
						.copied()
				});
				break node;
			};
			used += 1;
			match map.get(key) {
				Some(ExportValue::Range(rng)) => break rng.last().copied(),
				Some(ExportValue::Map(m)) => map = m,
				None => {
					let ExportMapKey::Named(name) = key else {
						unreachable!("keys are always named")
					};
					return Err(self.local_err(
						&MemberInfo::new(members[used - 1]).key_span,
						format!(
							"module {} has no export `{name}`",
							module_id.id
						),
					));
				}
			}
		};
		let at = MemberInfo::new(members[used - 1]).span;
		let Some(node) = node else {
			return Err(self.local_err(
				&at,
				"export has no value that can be resolved as a constant",
			));
		};
		let entry = Resolving::Export(module_id.id, keys[..used].to_vec());
		if resolving.contains(&entry) {
			return Err(
				self.local_err(&at, "export is defined in terms of itself")
			);
		}
		resolving.push(entry);
		let ret = Box::pin(p.resolve_node(node, resolving)).await;
		resolving.pop();
		let value = ret.map_err(|e| {
			self.local_err(
				&at,
				format!("failed to resolve export of module {}", module_id.id),
			)
			.s(e)
		})?;
		Ok((value, used))
	}
	/// Resolves a node from the raw export map
	#[expect(clippy::future_not_send)]
	async fn resolve_node(
		&self,
		node: AstKind<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		match node {
			AstKind::BindingIdentifier(ident) => {
				let sym = self.sym_id_of(ident).ok_or_else(|| {
					self.local_err(ident, "unresolved symbol")
				})?;
				self.resolve_symbol(sym, ident, resolving)
					.await
			}
			AstKind::IdentifierReference(ident) => {
				self.resolve_ident(ident, resolving)
					.await
			}
			AstKind::Function(_)
			| AstKind::ArrowFunctionExpression(_)
			| AstKind::Class(_) => Err(self.local_err(
				&node,
				"functions and classes are not constant values",
			)),
			_ => match self.expression_of(node) {
				Some(expr) => {
					Box::pin(self.resolve_value(expr, resolving)).await
				}
				None => Err(self.local_err(
					&node,
					format!(
						"unsupported export value of kind {}",
						node.debug_name()
					),
				)),
			},
		}
	}
	/// Gets the [`Expression`] that is `node`, by finding it in its parent
	fn expression_of(
		&self,
		node: AstKind<'ast>,
	) -> Option<&'ast Expression<'ast>> {
		let addr = node.address();
		let is_node = |e: &&'ast Expression<'ast>| e.address() == addr;
		let found = match self.p(node.node_id()) {
			AstKind::VariableDeclarator(d) => d.init.as_ref(),
			AstKind::ObjectProperty(p) => Some(&p.value),
			AstKind::AssignmentExpression(a) => Some(&a.right),
			AstKind::ArrowFunctionExpression(f) => f.get_expression(),
			AstKind::ExpressionStatement(s) => Some(&s.expression),
			AstKind::ReturnStatement(r) => r.argument.as_ref(),
			AstKind::ParenthesizedExpression(p) => Some(&p.expression),
			AstKind::PropertyDefinition(p) => p.value.as_ref(),
			AstKind::CallExpression(c) => c
				.arguments
				.iter()
				.filter_map(Argument::as_expression)
				.find(is_node),
			AstKind::ArrayExpression(a) => a
				.elements
				.iter()
				.filter_map(ArrayExpressionElement::as_expression)
				.find(is_node),
			AstKind::SequenceExpression(s) => {
				s.expressions.iter().find(is_node)
			}
			_ => None,
		};
		found.filter(is_node)
	}
	#[expect(clippy::future_not_send)]
	pub(super) async fn resolve_constant_value(
		&self,
		expr: &'ast Expression<'ast>,
	) -> PResult<SpannedValue> {
		self.resolve_value(expr, &mut Vec::new())
			.await
	}
	#[expect(clippy::future_not_send)]
	async fn resolve_value(
		&self,
		expr: &'ast Expression<'ast>,
		resolving: &mut Vec<Resolving>,
	) -> PResult<SpannedValue> {
		let ret = match expr {
			Expression::BooleanLiteral(lit) => SpannedValue {
				value: ConstantValue::Boolean(lit.value),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::NullLiteral(lit) => SpannedValue {
				value: ConstantValue::Null,
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::NumericLiteral(lit) => SpannedValue {
				value: ConstantValue::from(lit.value),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::BigIntLiteral(lit) => SpannedValue {
				value: ConstantValue::BigInt(lit.value.parse().unwrap()),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::StringLiteral(str) => SpannedValue {
				value: ConstantValue::String(str.to_string()),
				span: str.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::TemplateLiteral(tmpl) => {
				let mut ret = String::new();
				for (i, quasi) in tmpl.quasis.iter().enumerate() {
					if i != 0 {
						let expr = &tmpl.expressions[i - 1];
						let value =
							Box::pin(self.resolve_value(expr, resolving))
								.await?;
						let str_val = self.to_string(&value);
						ret.push_str(&str_val);
					}
					ret.push_str(quasi.value.cooked.as_ref().unwrap());
				}
				SpannedValue {
					value: ConstantValue::String(ret),
					span: tmpl.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::Identifier(ident) => {
				self.resolve_ident(ident, resolving)
					.await?
			}
			Expression::ArrayExpression(array_expression) => {
				let mut elements = Vec::new();
				for element in &array_expression.elements {
					match element {
						ArrayExpressionElement::SpreadElement(
							spread_element,
						) => {
							let spread_value = Box::pin(self.resolve_value(
								&spread_element.argument,
								resolving,
							))
							.await?;
							if let ConstantValue::Array(elts) =
								spread_value.value
							{
								elements.extend(elts);
							} else {
								return Err(self
									.local_err(
										&**spread_element,
										"spread element must be an array",
									)
									.s(self
										.remote_err(
											&spread_value.span,
											spread_value.module_id,
											"spread element value defined here",
										)
										.await));
							}
						}
						ArrayExpressionElement::Elision(hole) => {
							return Err(self.local_err(
								&**hole,
								"elisions are not supported",
							));
						}
						expr @ match_expression!(ArrayExpressionElement) => {
							let expr = expr.as_expression().unwrap();
							let value =
								Box::pin(self.resolve_value(expr, resolving))
									.await?;
							elements.push(value);
						}
					}
				}

				SpannedValue {
					value: ConstantValue::Array(elements),
					span: array_expression.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::AssignmentExpression(e) => match e.operator {
				AssignmentOperator::Assign => {
					let value =
						Box::pin(self.resolve_value(&e.right, resolving))
							.await?;
					SpannedValue {
						value: value.value,
						span: e.span,
						module_id: self.get_module_id()?.id,
					}
				}
				other => {
					return Err(self.local_err(
						&**e,
						format!(
							"assignment operator {other:?} is not supported for constant values"
						),
					));
				}
			},
			Expression::BinaryExpression(bin) => {
				let lhs =
					Box::pin(self.resolve_value(&bin.left, resolving)).await?;
				let rhs =
					Box::pin(self.resolve_value(&bin.right, resolving)).await?;
				match binary::fold_binary(bin.operator, &lhs.value, &rhs.value)
				{
					Ok(value) => SpannedValue {
						value,
						span: bin.span,
						module_id: self.get_module_id()?.id,
					},
					Err(msg) => {
						let rhs_err = self
							.remote_err(
								&rhs.span,
								rhs.module_id,
								"right operand defined here",
							)
							.await;
						let lhs_err = self
							.remote_err(
								&lhs.span,
								lhs.module_id,
								"left operand defined here",
							)
							.await
							.s(rhs_err);
						return Err(self.local_err(&**bin, msg).s(lhs_err));
					}
				}
			}
			Expression::CallExpression(call) => {
				Box::pin(self.resolve_call(call, resolving)).await?
			}
			Expression::ChainExpression(chain) => {
				Box::pin(self.resolve_chain(chain, resolving)).await?
			}
			Expression::ConditionalExpression(cond) => {
				let test =
					Box::pin(self.resolve_value(&cond.test, resolving)).await?;
				let expr = if test.value.is_truthy() {
					&cond.consequent
				} else {
					&cond.alternate
				};
				Box::pin(self.resolve_value(expr, resolving)).await?
			}
			Expression::LogicalExpression(expr) => {
				let lhs =
					Box::pin(self.resolve_value(&expr.left, resolving)).await?;
				macro_rules! rhs {
					() => {
						Box::pin(self.resolve_value(&expr.right, resolving))
							.await?
					};
				}
				match expr.operator {
					LogicalOperator::Or => {
						if lhs.value.is_truthy() {
							lhs
						} else {
							rhs!()
						}
					}
					LogicalOperator::And => {
						if lhs.value.is_falsy() {
							lhs
						} else {
							rhs!()
						}
					}
					LogicalOperator::Coalesce => {
						if lhs.value.is_nullish() {
							rhs!()
						} else {
							lhs
						}
					}
				}
			}
			Expression::NewExpression(new) => {
				Box::pin(self.resolve_new(new, resolving)).await?
			}
			Expression::ObjectExpression(obj) => {
				Box::pin(self.resolve_object(obj, resolving)).await?
			}
			Expression::ParenthesizedExpression(e) => {
				Box::pin(self.resolve_value(&e.expression, resolving)).await?
			}
			Expression::SequenceExpression(seq) => {
				let last = seq.expressions.last().unwrap();
				Box::pin(self.resolve_value(last, resolving)).await?
			}
			Expression::TaggedTemplateExpression(
				tagged_template_expression,
			) => {
				return Err(self.local_err(
					&**tagged_template_expression,
					"tagged template expressions are not constant values",
				));
			}
			Expression::UnaryExpression(unary) => {
				let value = match (
					unary.operator,
					unary.argument.without_parentheses(),
				) {
					// the operand doesn't need to be constant
					(UnaryOperator::Void, _) => ConstantValue::Undefined,
					(
						UnaryOperator::Typeof,
						Expression::FunctionExpression(_)
						| Expression::ArrowFunctionExpression(_)
						| Expression::ClassExpression(_),
					) => ConstantValue::String("function".to_owned()),
					_ => {
						let arg = Box::pin(
							self.resolve_value(&unary.argument, resolving),
						)
						.await?;
						match unary::fold_unary(unary.operator, &arg.value) {
							Ok(value) => value,
							Err(msg) => {
								return Err(self.local_err(&**unary, msg).s(
									self.remote_err(
										&arg.span,
										arg.module_id,
										"operand defined here",
									)
									.await,
								));
							}
						}
					}
				};
				SpannedValue {
					value,
					span: unary.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::ComputedMemberExpression(member) => {
				Box::pin(
					self.resolve_member(
						MemberExprRef::Computed(member),
						resolving,
					),
				)
				.await?
			}
			Expression::StaticMemberExpression(member) => {
				Box::pin(
					self.resolve_member(
						MemberExprRef::Static(member),
						resolving,
					),
				)
				.await?
			}
			Expression::RegExpLiteral(regex) => {
				return Err(self.local_err(
					&**regex,
					"regular expressions are not supported as constant values",
				));
			}
			Expression::ArrowFunctionExpression(_)
			| Expression::FunctionExpression(_) => {
				return Err(self.local_err(
					expr,
					"function expressions are not constant values",
				));
			}
			other => {
				return Err(self.local_err(
					other,
					"unsupported expression type for constant value",
				));
			}
		};
		Ok(ret)
	}
}

mod binary;
mod member;
mod primitive;
#[cfg(test)]
mod tests;
mod unary;
