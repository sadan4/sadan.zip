use std::{
	borrow::Cow,
	sync::{Arc, OnceLock},
	time::Instant,
};

use anyhow::{Context as _, Result, bail};
use bitflags::bitflags;
use clap::{ArgMatches, CommandFactory, FromArgMatches};
use futures_core::future::BoxFuture;
use itertools::Itertools;
use serenity::{
	all::{
		Context,
		CreateInteractionResponse,
		CreateInteractionResponseMessage,
		EventHandler,
		GuildId,
		Interaction,
		Message,
		ReactionType,
		prelude::{TypeMap, TypeMapKey},
	},
	async_trait,
	futures::future,
};
use shlex::Shlex;
use tokio::sync::{Mutex, OnceCell, RwLock};
use tracing::{error, info};

mod ctx;
mod slash;

pub use check::{Check, OWNER, Status};
pub use ctx::CommandCtx;
pub use slash::{SlashArg, SlashSchema, SlashSchemaFn};

use crate::{
	BotConfig,
	util::{MESSAGE_RECEIVE_TIME, REFERENCED_USER, get_ref_user},
};

mod check;

pub struct CommandFramework {
	root_cmd: &'static Command,
	prefixes: Vec<Cow<'static, str>>,
	/// When set, slash commands are registered to this guild (instant
	/// propagation); otherwise they are registered globally.
	guild: Option<GuildId>,
	data: Arc<Mutex<TypeMap>>,
}

impl CommandFramework {
	pub fn new(root_cmd: &'static Command) -> Self {
		Self {
			root_cmd,
			prefixes: Vec::new(),
			guild: None,
			data: Arc::new(Mutex::new(TypeMap::new())),
		}
	}

	pub async fn get_data<T>(&self) -> Option<T::Value>
	where
		T: TypeMapKey,
		T::Value: Clone,
	{
		self.data
			.lock()
			.await
			.get::<T>()
			.cloned()
	}

	pub async fn set_data<T>(&self, value: T::Value)
	where
		T: TypeMapKey,
	{
		self.data
			.lock()
			.await
			.insert::<T>(value);
	}
	/// Register slash commands to a single guild (instant propagation, ideal
	/// for development) instead of globally.
	pub fn with_guild(mut self, guild: impl Into<Option<GuildId>>) -> Self {
		self.guild = guild.into();
		self
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
		let mut parser = Shlex::new(msg);
		let components = parser.by_ref().collect_vec();
		if parser.had_error {
			bail!("Failed to tokenize message content");
		}
		self.match_command_tokens(components)
	}

	/// Resolve an already-tokenized command line (the multicall argv: leading
	/// token is the top-level command name) to a command and its parsed
	/// arguments. This is the shared back-end for both prefix text (after
	/// shlex tokenization) and slash invocations (tokens synthesized from the
	/// interaction options).
	pub fn match_command_tokens<I, T>(
		&self,
		tokens: I,
	) -> Result<(&Command, ArgMatches)>
	where
		I: IntoIterator<Item = T>,
		T: Into<std::ffi::OsString> + Clone,
	{
		let root_cmd = self
			.root_cmd
			.parser
			.get(self.root_cmd)
			.clone();
		let matches = root_cmd
			.try_get_matches_from(tokens)
			.context("Failed to find matching command")?;
		let (cmd, arg_matches) =
			Self::walk_command_tree(self.root_cmd, &matches)
				.context("Failed to walk command tree")?;
		Ok((cmd, arg_matches.clone()))
	}

	/// Resolve `content` to a command, enforce that it is invocable in the
	/// current context, and run its executor with the given [`CommandCtx`].
	async fn execute_command_inner(
		&self,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
		content: &str,
		required: Availability,
	) -> Result<()> {
		let (cmd, args) = self
			.find_matching_command(content)
			.context("Invalid command")?;
		if !cmd.availability.contains(required) {
			bail!("command is not available in this context");
		}
		if let Status::Fail(reason) = self.run_checks(ctx, cctx, cmd).await? {
			self.handle_check_failure(ctx, cctx, cmd, &reason)
				.await?;
			return Ok(());
		}
		let e = &cmd.executor;
		e.execute(ctx, cctx, cmd, self, &args)
			.await
			.with_context(|| {
				format!("Failed to execute command {}", cmd.names[0])
			})?;
		Ok(())
	}

	/// Run every check gating `cmd` in order, short-circuiting on the first
	/// failure. A check erroring (as opposed to cleanly failing) aborts
	/// dispatch and surfaces through the normal error path.
	async fn run_checks(
		&self,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
		cmd: &Command,
	) -> Result<Status> {
		for check in cmd.checks {
			match (check.func)(ctx, cctx, cmd, self)
				.await
				.with_context(|| format!("check `{}` errored", check.name))?
			{
				Status::Pass => {}
				fail @ Status::Fail(_) => return Ok(fail),
			}
		}
		Ok(Status::Pass)
	}

	/// A check refused the invocation: log the operator-facing reason and, when
	/// one is provided, tell the invoking user why.
	async fn handle_check_failure(
		&self,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
		_: &Command,
		check: &Check,
	) -> Result<()> {
		match cctx {
			CommandCtx::Prefix { msg } => {
				if let Err(e) = msg
					.react(ctx, ReactionType::Unicode(String::from("❌")))
					.await
				{
					info!(%msg.channel_id, ?msg.guild_id, %msg.id, "Failed to react, {e:?}");
				}
			}
			CommandCtx::Application { interaction } => {
				let failed_msg = if check.name == OWNER.name {
					"Only the bot owner can use this command.".into()
				} else {
					format!("Check `{}` failed for this command.", check.name)
				};
				let res = CreateInteractionResponse::Message(
					CreateInteractionResponseMessage::new()
						.ephemeral(true)
						.content(failed_msg),
				);
				if let Err(e) = interaction
					.create_response(ctx, res)
					.await
				{
					info!(
						"Failed to respond to interaction with check failure message: {:?}",
						e
					);
				}
			}
		}
		Ok(())
	}

	/// Eagerly construct the session context of every command (recursively)
	/// flagged `#[early_init]`, so their first invocation pays no init cost.
	async fn preload_eager_commands(&self, cmd: &Command) -> Result<()> {
		if cmd
			.flags
			.contains(CommandFlags::EAGER_INIT)
		{
			cmd.executor
				.ensure_init(self)
				.await
				.with_context(|| {
					format!("failed to eagerly init command {}", cmd.names[0])
				})?;
		}
		for sub in cmd.sub_cmds {
			Box::pin(self.preload_eager_commands(sub)).await?;
		}
		Ok(())
	}

	async fn should_use_ansi(ctx: &Context) -> bool {
		let lock = ctx.data.read().await;
		let config = lock.get::<BotConfig>().unwrap();
		config.use_ansi_clap_errors
	}

	pub async fn execute_command(&self, ctx: &Context, msg: &Message) -> () {
		for prefix in &self.prefixes {
			if msg.content.starts_with(prefix.as_ref()) {
				let cctx = CommandCtx::Prefix { msg };
				let content = &msg.content[prefix.len()..];
				if let Err(e) = self
					.execute_command_inner(
						ctx,
						&cctx,
						content,
						Availability::PREFIX,
					)
					.await
				{
					if let Some(e) = e.downcast_ref::<clap::Error>() {
						let use_ansi = Self::should_use_ansi(ctx).await;
						let msg_content = render_clap_error(e, use_ansi);
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

/// Render a clap parse/usage error into a Discord-ready, length-capped ANSI
/// code block.
fn render_clap_error(e: &clap::Error, use_ansi: bool) -> String {
	let mut rendered = if use_ansi {
		format!("{}", e.render().ansi())
	} else {
		format!("{}", e.render())
	};
	// FIXME: slice the bytes instead of popping/removing to avoid O(n) copy operations
	while rendered.bytes().next_back() == Some(b'\n') {
		rendered.pop();
	}
	while rendered.bytes().next() == Some(b'\n') {
		rendered.remove(0);
	}
	crate::util::wrap_code_block(&rendered, "ansi")
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

	async fn ready(&self, ctx: Context, _ready: serenity::all::Ready) -> () {
		if let Err(e) = self.register_slash_commands(&ctx).await {
			error!("Failed to register slash commands: {:?}", e);
		}
		if let Err(e) = self
			.preload_eager_commands(self.root_cmd)
			.await
		{
			error!("Failed to preload eager commands: {:?}", e);
		}
	}

	async fn interaction_create(
		&self,
		ctx: Context,
		interaction: Interaction,
	) -> () {
		let Interaction::Command(command) = interaction else {
			return;
		};
		let handler_timestamp = Instant::now();
		let fut = self.handle_interaction(&ctx, &command);
		// a slash command has no referenced message, so `FROM_REPLY`-style
		// defaults have nothing to resolve against
		REFERENCED_USER
			.scope(None, MESSAGE_RECEIVE_TIME.scope(handler_timestamp, fut))
			.await;
	}
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

bitflags! {
	/// The invocation front-ends a command is exposed through. Commands default
	/// to being available both ways; `#[prefix_only]` / `#[slash_only]` narrow
	/// this.
	#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
	pub struct Availability: u8 {
		/// Invocable as prefix text (e.g. `;ping`).
		const PREFIX = 1 << 0;
		/// Invocable as a Discord application (slash) command.
		const SLASH = 1 << 1;
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
		cctx: &CommandCtx<'_>,
		cmd: &Command,
		fw: &CommandFramework,
		args: &ArgMatches,
	) -> Result<()>;
}

type ExecutorFactory = for<'fut> fn(
	&'fut CommandFramework,
) -> BoxFuture<
	'fut,
	Result<Box<dyn CommandExecutor + Send + Sync>>,
>;
pub struct OpaqueExecutor(
	OnceCell<Box<dyn CommandExecutor + Send + Sync>>,
	ExecutorFactory,
);

impl OpaqueExecutor {
	pub async fn execute(
		&self,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
		cmd: &Command,
		fw: &CommandFramework,
		args: &ArgMatches,
	) -> Result<()> {
		let executor = self
			.0
			.get_or_try_init(|| (self.1)(fw))
			.await?;
		executor
			.execute(ctx, cctx, cmd, fw, args)
			.await?;
		Ok(())
	}
	/// Force the session context to be constructed now (if not already),
	/// discarding the executor handle. Used to honor `#[early_init]`.
	pub async fn ensure_init(&self, fw: &CommandFramework) -> Result<()> {
		self.0
			.get_or_try_init(|| (self.1)(fw))
			.await?;
		Ok(())
	}
	pub const fn from_const(factory: ExecutorFactory) -> Self {
		Self(OnceCell::const_new(), factory)
	}

	pub fn dummy_executor() -> impl CommandExecutor + Send + Sync + 'static {
		struct Todo;
		#[async_trait]
		impl CommandExecutor for Todo {
			async fn execute(
				&self,
				_: &Context,
				_: &CommandCtx<'_>,
				_: &Command,
				_: &CommandFramework,
				_: &ArgMatches,
			) -> Result<()> {
				bail!("CommandExecutor not implemented for this command")
			}
		}
		Todo
	}

	pub const fn __todo() -> Self {
		Self::from_const(|_| {
			Box::pin(future::ready(Ok(Box::new(Self::dummy_executor()) as Box<dyn CommandExecutor + Send + Sync>)))
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
	pub availability: Availability,
	/// Native Discord option types for this command's arguments, when they opt
	/// into typing via `#[slash_args]`; `None` means all options register as
	/// `String` and are re-parsed by clap.
	pub slash_schema: Option<SlashSchemaFn>,
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
