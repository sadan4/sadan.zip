use super::*;
use macros::test;

fn dep_ids(deps: &[(ModuleId, SpanDumper<'_>)]) -> Vec<u32> {
	deps.iter()
		.map(|(id, _)| id.0)
		.collect()
}

/// `wreq.t.bind(wreq, id, mode)` is used by intl chunk loaders
#[test]
fn finds_wreq_t_bind_lazy_deps() {
	let p = parse!("test_data/wp/deps/lazyTBind.js");
	let (_, lazy) = p.dbg_outgoing_deps();
	assert_eq!(dep_ids(&lazy), vec![64272, 100252, 131532, 568719, 817596]);
}

#[test]
fn wreq_t_bind_does_not_leak_into_sync_deps() {
	let p = parse!("test_data/wp/deps/lazyTBind.js");
	let (sync, _) = p.dbg_outgoing_deps();
	assert_eq!(dep_ids(&sync), vec![632296]);
}

#[test]
fn finds_sync_dep_behind_spread() {
	let p = parse!("test_data/wp/deps/syncSpread.js");
	let (sync, _) = p.dbg_outgoing_deps();
	assert_eq!(dep_ids(&sync), vec![75255]);
}
