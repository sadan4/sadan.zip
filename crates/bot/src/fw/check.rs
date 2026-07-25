use anyhow::Result;
use futures_core::future::BoxFuture;
use serenity::all::Context;
use tracing::info;

use crate::BotConfig;

use super::{CommandCtx, CommandFramework};

pub enum Status {
	Pass,
	Fail(Check),
}

pub type CheckResult = Result<Status>;

pub type CheckFn = for<'fut> fn(
	&'fut Context,
	&'fut CommandCtx<'fut>,
	&'fut super::Command,
	&'fut CommandFramework,
) -> BoxFuture<'fut, CheckResult>;

pub struct Check {
	pub name: &'static str,
	pub func: CheckFn,
	pub check_for_help: bool,
	pub hide_check: bool,
}

/// Gate a command behind bot ownership: passes only for the application's owner
/// (or, for team-owned apps, the team owner). Attach with
/// `#[checks(crate::fw::OWNER)]`.
pub const OWNER: Check = Check {
	name: "owner",
	func: is_owner,
	check_for_help: true,
	hide_check: false,
};

fn is_owner<'fut>(
	ctx: &'fut Context,
	cctx: &'fut CommandCtx<'fut>,
	_cmd: &'fut super::Command,
	_fw: &'fut CommandFramework,
) -> BoxFuture<'fut, CheckResult> {
	Box::pin(async move {
		let lock = ctx.data.read().await;
		let config = lock.get::<BotConfig>().unwrap();
		let author = cctx.author();
		let is_owner = config.bot_owners.contains(&author.id);
		Ok(if is_owner {
			Status::Pass
		} else {
			info!(
				"owner check failed for user `{}` (id {})",
				author.name, author.id
			);
			Status::Fail(OWNER)
		})
	})
}
