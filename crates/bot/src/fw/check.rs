use anyhow::Result;
use futures_core::future::BoxFuture;
use serenity::all::Context;
use tracing::info;

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
}

/// Only allow the owner(s) of the bot to run this command.
///
/// Use with `#[checks(crate::fw::OWNER)]`.
pub const OWNER: Check = Check {
	name: "owner",
	func: is_owner,
};

fn is_owner<'fut>(
	_ctx: &'fut Context,
	cctx: &'fut CommandCtx<'fut>,
	_cmd: &'fut super::Command,
	fw: &'fut CommandFramework,
) -> BoxFuture<'fut, CheckResult> {
	Box::pin(async move {
		let author = cctx.author();
		let is_owner = fw
			.config
			.bot_owners
			.contains(&author.id);
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
