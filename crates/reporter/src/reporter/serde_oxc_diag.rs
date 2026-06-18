use std::borrow::Cow;

use miette::{LabeledSpan, Labels};
use oxc::diagnostics::{OxcCode, OxcDiagnostic, OxcDiagnosticInner};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct OwnCode {
	scope: Option<Cow<'static, str>>,
	number: Option<Cow<'static, str>>,
}

#[derive(Serialize, Deserialize)]
struct OwnOxcDiagnostic {
	message: Cow<'static, str>,
	labels: Vec<LabeledSpan>,
	help: Option<Cow<'static, str>>,
	note: Option<Cow<'static, str>>,
	severity: miette::Severity,
	code: OwnCode,
	url: Option<Cow<'static, str>>,
}

impl From<&OxcDiagnostic> for OwnOxcDiagnostic {
	fn from(value: &OxcDiagnostic) -> Self {
		let OxcDiagnosticInner {
			message,
			labels,
			help,
			note,
			severity,
			code: OxcCode { scope, number },
			url,
		} = &**value;
		Self {
			message: message.clone(),
			labels: labels.to_vec(),
			help: help.clone(),
			note: note.clone(),
			severity: *severity,
			code: OwnCode {
				scope: scope.clone(),
				number: number.clone(),
			},
			url: url.clone(),
		}
	}
}

impl From<OwnOxcDiagnostic> for OxcDiagnosticInner {
	fn from(val: OwnOxcDiagnostic) -> Self {
		Self {
			code: OxcCode {
				scope: val.code.scope,
				number: val.code.number,
			},
			message: val.message,
			labels: Labels::from_iter(val.labels),
			help: val.help,
			note: val.note,
			severity: val.severity,
			url: val.url,
		}
	}
}

pub fn serialize<S: Serializer>(
	t: &OxcDiagnostic,
	s: S,
) -> Result<S::Ok, S::Error> {
	let own: OwnOxcDiagnostic = t.into();
	own.serialize(s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
	d: D,
) -> Result<Box<OxcDiagnostic>, D::Error> {
	let own = OwnOxcDiagnostic::deserialize(d)?;
	let inner = own.into();
	// there is now way to create an OxcDiagnostic from an OxcDiagnosticInner
	// despite OxcDiagnostic only holding an OxcDiagnosticInner
	// so we create a dummy one as cheaply as possible then
	// set the inner pointer because it implements DerefMut<OxcDiagnosticInner>
	let mut tmp = OxcDiagnostic::warn("");
	*tmp = inner;
	Ok(Box::new(tmp))
}
