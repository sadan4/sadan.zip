use std::{borrow::Cow, sync::OnceLock, time::Instant};

use anyhow::{Context as _, Result, bail};
use bitflags::bitflags;
use clap::{ArgMatches, CommandFactory, FromArgMatches};
use futures_core::future::BoxFuture;
use itertools::Itertools;
use serenity::{
	all::{Context, EventHandler, Message},
	async_trait,
	futures::future,
};
use shlex::Shlex;
use tokio::sync::OnceCell;
use tracing::{error, info};

use crate::{
	fw::check::Check,
	util::{MESSAGE_RECEIVE_TIME, REFERENCED_USER, get_ref_user},
};

mod check {
	use anyhow::Result;
	use futures_core::future::BoxFuture;
	use serenity::all::{Context, Message};

	pub enum Reason {
		Unknown,
		User(String),
		Log(String),
		UserAndLog { user: String, log: String },
	}

	pub enum Status {
		Pass,
		Fail(Reason),
	}

	pub type CheckResult = Result<Status>;

	pub type CheckFn = for<'fut> fn(
		&'fut Context,
		&'fut Message,
		&'fut super::Command,
		&'fut super::CommandFramework,
	) -> BoxFuture<'fut, CheckResult>;

	pub struct Check {
		pub name: &'static str,
		pub func: CheckFn,
		pub check_for_help: bool,
		pub hide_check: bool,
	}
}

pub struct CommandFramework {
	root_cmd: &'static Command,
	prefixes: Vec<Cow<'static, str>>,
}

impl CommandFramework {
	pub const fn new(root_cmd: &'static Command) -> Self {
		Self {
			root_cmd,
			prefixes: Vec::new(),
		}
	}

	pub fn with_prefixes<T, I>(mut self, prefixes: I) -> Self
	where
		T: Into<Cow<'static, str>>,
		I: Iterator<Item = T>,
	{
		self.prefixes
			.extend(prefixes.map(Into::into));
		self
	}

	pub fn with_prefix<T>(mut self, prefix: T) -> Self
	where
		T: Into<Cow<'static, str>>,
	{
		self.prefixes.push(prefix.into());
		self
	}

	fn walk_command_tree<'a, 'b>(
		parent: &'a Command,
		matches: &'b ArgMatches,
	) -> Result<(&'a Command, &'b ArgMatches)> {
		let Some((name, args)) = matches.subcommand() else {
			return Ok((parent, matches));
		};
		for sub_cmd in parent.sub_cmds {
			for sub_name in sub_cmd.names {
				if *sub_name == name {
					return Self::walk_command_tree(sub_cmd, args);
				}
			}
		}
		if parent.flags & CommandFlags::ROOT_GROUP != CommandFlags::NONE {
			bail!("No matching command found");
		}
		Ok((parent, matches))
	}
	pub fn find_matching_command(
		&self,
		msg: &str,
	) -> Result<(&Command, ArgMatches)> {
		let root_cmd = self
			.root_cmd
			.parser
			.get(self.root_cmd)
			.clone();
		let mut parser = Shlex::new(msg);
		let components = parser.by_ref().collect_vec();
		if parser.had_error {
			bail!("Failed to tokenize message content");
		}
		let matches = root_cmd
			.try_get_matches_from(&components)
			.context("Failed to find matching command")?;
		let (cmd, arg_matches) =
			Self::walk_command_tree(self.root_cmd, &matches)
				.context("Failed to walk command tree")?;
		Ok((cmd, arg_matches.clone()))
	}

	async fn execute_command_inner(
		&self,
		ctx: &Context,
		msg: &Message,
		prefix: &str,
	) -> Result<()> {
		let (cmd, args) = self
			.find_matching_command(&msg.content[prefix.len()..])
			.context("Invalid command")?;
		let e = &cmd.executor;
		e.execute(ctx, msg, cmd, self, &args)
			.await
			.with_context(|| {
				format!("Failed to execute command {}", cmd.names[0])
			})?;
		Ok(())
	}
	pub async fn execute_command(&self, ctx: &Context, msg: &Message) -> () {
		for prefix in &self.prefixes {
			if msg.content.starts_with(prefix.as_ref()) {
				if let Err(e) = self
					.execute_command_inner(ctx, msg, prefix)
					.await
				{
					if let Some(e) = e.downcast_ref::<clap::Error>() {
						let mut rendered = format!("{}", e.render().ansi());
						// FIXME: slice the bytes instead of popping/removing to avoid O(n) copy operations
						while rendered.bytes().next_back() == Some(b'\n') {
							rendered.pop();
						}
						while rendered.bytes().next() == Some(b'\n') {
							rendered.remove(0);
						}
						const HEADER: &str = "```ansi\n";
						const FOOTER: &str = "\n```";
						const MAX_LEN: usize = 2000;
						// resets any style left open by the cut, then marks the
						// message as incomplete
						const ELLIPSIS: &str = "\u{1b}[0m…";
						// they are all ansi only
						const MAX_RENDERED_CODEPOINTS: usize =
							MAX_LEN - HEADER.len() - FOOTER.len();
						if rendered.chars().count() > MAX_RENDERED_CODEPOINTS {
							let budget = MAX_RENDERED_CODEPOINTS
								- ELLIPSIS.chars().count();
							rendered.truncate(ansi_truncation_point(
								&rendered, budget,
							));
							rendered.push_str(ELLIPSIS);
						}
						let mut msg_content =
							String::with_capacity(MAX_LEN.min(
								HEADER.len() + rendered.len() + FOOTER.len(),
							));
						msg_content.push_str(HEADER);
						msg_content.push_str(&rendered);
						msg_content.push_str(FOOTER);
						if let Err(e) = msg
							.reply_ping(&ctx.http, msg_content)
							.await
						{
							error!("Failed to send error message: {:?}", e);
						}
					} else {
						error!("Failed to execute command: {:?}", e);
					}
				} else {
					info!("Executed command");
				}
				break;
			}
		}
	}
}

#[async_trait]
impl EventHandler for CommandFramework {
	async fn message(&self, ctx: Context, msg: Message) -> () {
		let handler_timestamp = Instant::now();
		let ref_user = get_ref_user(&msg);
		let fut = self.execute_command(&ctx, &msg);
		REFERENCED_USER
			.scope(ref_user, MESSAGE_RECEIVE_TIME.scope(handler_timestamp, fut))
			.await;
	}
}

/// Returns the byte index at which `s` can be truncated so that what remains
/// is at most `max` codepoints long.
///
/// A cut that would land inside an ANSI escape sequence is moved back to the
/// start of that sequence, so the tail of a sequence is never left behind to
/// be rendered as literal text.
fn ansi_truncation_point(s: &str, max: usize) -> usize {
	/// Where in an escape sequence the scanner currently is.
	enum State {
		Text,
		/// An escape has been seen, but not the byte that says what kind of
		/// sequence it introduces.
		Escape,
		/// Inside a control sequence, waiting on its final byte.
		Csi,
	}
	let mut state = State::Text;
	// the start of the escape sequence being scanned, if any
	let mut escape_start = 0;
	for (count, (idx, ch)) in s.char_indices().enumerate() {
		if count == max {
			return match state {
				State::Text => idx,
				State::Escape | State::Csi => escape_start,
			};
		}
		state = match state {
			State::Text if ch == '\u{1b}' => {
				escape_start = idx;
				State::Escape
			}
			State::Escape if ch == '[' => State::Csi,
			// a control sequence ends on a final byte in this range;
			// everything before it is a parameter or intermediate byte
			State::Csi if matches!(ch, '\u{40}'..='\u{7e}') => State::Text,
			State::Csi => State::Csi,
			// anything other than a `[` after the escape is a two character
			// sequence, which this character terminates
			State::Text | State::Escape => State::Text,
		};
	}
	s.len()
}

bitflags! {
	pub struct UsageLocation: u8 {
		const NONE = 0;
		const DM = 1 << 0;
		const GUILD = 1 << 1;

	}
}

bitflags! {
	#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
	pub struct CommandFlags: u8 {
		const NONE = 0;
		const EAGER_INIT = 1 << 0;
		const GROUP = 1 << 1;
		const ROOT_GROUP = 1 << 2;
	}
}

#[async_trait]
#[diagnostic::on_unimplemented(
	message = "the trait bound `{Self}: bot::fw::CommandExecutor` is not satisfied",
	note = "for local types consider adding `#[macros::executor]` to your command handler"
)]
pub trait CommandExecutor {
	async fn execute(
		&self,
		ctx: &Context,
		msg: &Message,
		cmd: &Command,
		fw: &CommandFramework,
		args: &ArgMatches,
	) -> Result<()>;
}

type ExecutorFactory =
	for<'fut> fn(
		&'fut CommandFramework,
	) -> BoxFuture<'fut, Box<dyn CommandExecutor + Send + Sync>>;
pub struct OpaqueExecutor(
	OnceCell<Box<dyn CommandExecutor + Send + Sync>>,
	ExecutorFactory,
);

impl OpaqueExecutor {
	pub async fn execute(
		&self,
		ctx: &Context,
		msg: &Message,
		cmd: &Command,
		fw: &CommandFramework,
		args: &ArgMatches,
	) -> Result<()> {
		let executor = self
			.0
			.get_or_init(|| (self.1)(fw))
			.await;
		executor
			.execute(ctx, msg, cmd, fw, args)
			.await?;
		Ok(())
	}
	pub const fn from_const(factory: ExecutorFactory) -> Self {
		Self(OnceCell::const_new(), factory)
	}
	pub const fn __todo() -> Self {
		struct Todo;
		#[async_trait]
		impl CommandExecutor for Todo {
			async fn execute(
				&self,
				_: &Context,
				&_: &Message,
				_: &Command,
				_: &CommandFramework,
				_: &ArgMatches,
			) -> Result<()> {
				bail!("CommandExecutor not implemented for this command")
			}
		}
		Self::from_const(|_| {
			Box::pin(future::ready(Box::new(Todo)
				as Box<dyn CommandExecutor + Send + Sync + 'static>))
		})
	}
}

pub struct Command {
	pub checks: &'static [Check],
	pub names: &'static [&'static str],
	pub parser: ParserFactory,
	pub desc: Option<&'static str>,
	pub usage_location: UsageLocation,
	pub sub_cmds: &'static [&'static Self],
	pub executor: OpaqueExecutor,
	pub flags: CommandFlags,
}

type ParserFactoryFn = for<'fut> fn(&'fut Command) -> clap::Command;

pub struct ParserFactory {
	parser: OnceLock<clap::Command>,
	factory: ParserFactoryFn,
}

impl ParserFactory {
	pub fn get(&self, cmd: &Command) -> &clap::Command {
		self.parser
			.get_or_init(|| (self.factory)(cmd))
	}
	pub const fn from_fn(f: ParserFactoryFn) -> Self {
		Self {
			parser: OnceLock::new(),
			factory: f,
		}
	}

	fn null_parser(cmd: &Command) -> clap::Command {
		let is_root =
			cmd.flags & CommandFlags::ROOT_GROUP != CommandFlags::NONE;
		let cmd_name = if is_root {
			""
		} else {
			debug_assert!(
				cmd.names.len() == 1,
				"TODO: handle commands with more than one name"
			);
			cmd.names[0]
		};
		let mut command =
			clap::Command::new(cmd_name).about(cmd.desc.unwrap_or_default());
		if is_root {
			command = command.multicall(true);
		}
		for sub in cmd.sub_cmds {
			let sub_cmd = sub.parser.get(sub);
			command = command.subcommand(sub_cmd);
		}
		command
	}

	fn parser<T>(cmd: &Command) -> clap::Command
	where
		T: CommandFactory + FromArgMatches + 'static,
	{
		let cmd_name =
			if cmd.flags & CommandFlags::ROOT_GROUP == CommandFlags::NONE {
				debug_assert!(
					cmd.names.len() == 1,
					"TODO: handle commands with more than one name"
				);
				cmd.names[0]
			} else {
				""
			};
		let mut command = T::command()
			.name(cmd_name)
			.about(cmd.desc.unwrap_or_default());
		if !cmd.sub_cmds.is_empty() {
			// the command's own args and a subcommand are mutually
			// exclusive: supplying a subcommand waives the parent's required
			// args, and supplying both is an error
			command = command.args_conflicts_with_subcommands(true);
		}
		for sub in cmd.sub_cmds {
			let sub_cmd = sub.parser.get(sub);
			command = command.subcommand(sub_cmd);
		}
		command
	}

	pub const fn __make_parser<T>() -> Self
	where
		T: 'static + clap::CommandFactory + clap::FromArgMatches,
	{
		Self::from_fn(Self::parser::<T>)
	}

	pub const fn __make_null_parser() -> Self {
		Self::from_fn(Self::null_parser)
	}
}

#[cfg(test)]
mod tests;
