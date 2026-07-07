use oxc::span::Span;
use webpack_ast_parser::export_map::{
	ExportMap,
	ExportRange,
	ExportValue,
	ExtraData,
	RangeExportRange,
	StoreData,
};

fn store_data(StoreData { flux_events, .. }: StoreData<Span>) -> StoreData<()> {
	StoreData {
		store: (),
		flux_events: flux_events
			.into_iter()
			.map(|(k, _)| (k, ()))
			.collect(),
	}
}
fn extra_data(data: ExtraData<Span>) -> ExtraData<()> {
	use ExtraData as ED;
	match data {
		ED::None => ED::None,
		ED::Store(sd) => ED::Store(store_data(sd)),
	}
}
fn range(ExportRange(arr, hov): RangeExportRange) -> ExportRange<()> {
	ExportRange(vec![(); arr.len()], hov)
}
fn value(value: ExportValue<Span>) -> ExportValue<()> {
	use ExportValue as EV;
	match value {
		EV::Range(v) => EV::Range(range(v)),
		EV::Map(v) => EV::Map(map(v)),
	}
}
pub fn map(
	ExportMap {
		exports,
		cjs_default,
		hover,
		extra_data: ed,
	}: ExportMap<Span>,
) -> ExportMap<()> {
	ExportMap {
		exports: exports
			.into_iter()
			.map(|(k, v)| (k, value(v)))
			.collect(),
		cjs_default: cjs_default.map(|v| Box::new(value(*v))),
		hover,
		extra_data: extra_data(ed),
	}
}
