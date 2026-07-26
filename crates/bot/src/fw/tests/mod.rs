use super::*;

#[test]
fn cuts_plain_text_at_a_codepoint_boundary() {
	let s = "héllo";
	// the budget counts codepoints, not the bytes the é takes up
	assert_eq!(crate::util::ansi_truncation_point(s, 5), s.len());
	assert_eq!(crate::util::ansi_truncation_point(s, 9), s.len());
	assert_eq!(&s[..crate::util::ansi_truncation_point(s, 2)], "hé");
}

#[test]
fn never_cuts_inside_a_control_sequence() {
	let s = "ab\u{1b}[1;31mcd";
	// every cut that would land inside the sequence falls back to its
	// start
	for max in 2..=8 {
		assert_eq!(&s[..crate::util::ansi_truncation_point(s, max)], "ab");
	}
	assert_eq!(
		&s[..crate::util::ansi_truncation_point(s, 9)],
		"ab\u{1b}[1;31m"
	);
}

#[test]
fn never_cuts_inside_a_two_character_escape() {
	let s = "a\u{1b}Mb";
	assert_eq!(&s[..crate::util::ansi_truncation_point(s, 1)], "a");
	assert_eq!(&s[..crate::util::ansi_truncation_point(s, 2)], "a");
	assert_eq!(&s[..crate::util::ansi_truncation_point(s, 3)], "a\u{1b}M");
}

mod command_framework {
	use super::*;
	use std::{
		future,
		sync::atomic::{AtomicU64, Ordering},
	};

	use anyhow::Result;
	use clap::{FromArgMatches, Parser};
	use macros::{command, executor};
	use serenity::all::{Context, UserId};

	use crate::util::{FROM_REPLY, REFERENCED_USER, UserArg};

	#[derive(Parser)]
	struct EchoArgs {
		#[arg(long)]
		text: String,
		#[arg(long, default_value_t = 1)]
		count: u8,
	}

	#[derive(Parser)]
	struct RefArgs {
		#[arg(default_value = FROM_REPLY)]
		user: UserArg,
	}

	#[command]
	#[arg_parser = RefArgs]
	async fn refcmd(
		args: RefArgs,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
	) -> Result<()> {
		let _ = (args, ctx, cctx);
		Ok(())
	}

	#[command]
	#[arg_parser = EchoArgs]
	async fn echo(
		args: EchoArgs,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
	) -> Result<()> {
		let _ = (args, ctx, cctx);
		Ok(())
	}

	#[command]
	async fn ping(ctx: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
		let _ = (ctx, cctx);
		Ok(())
	}

	#[command]
	#[sub_cmds(echo)]
	#[group]
	struct Nested;

	#[command]
	#[sub_cmds(nested)]
	#[group]
	struct Outer;

	// a stateless callable group: takes its own args, has a subcommand, and
	// runs its own handler when no subcommand matches
	#[command]
	#[group]
	#[arg_parser = EchoArgs]
	#[sub_cmds(ping)]
	async fn grp(
		args: EchoArgs,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
	) -> Result<()> {
		let _ = (args, ctx, cctx);
		Ok(())
	}

	// a stateful callable group: its session context is built once via `#[init]`
	#[command]
	#[init = Counter::new]
	#[sub_cmds(ping)]
	struct Counter {
		n: AtomicU64,
	}

	impl Counter {
		fn new(_: &CommandFramework) -> future::Ready<Result<Self>> {
			future::ready(Ok(Self {
				n: AtomicU64::new(0),
			}))
		}
	}

	#[executor]
	async fn counter(
		this: &Counter,
		ctx: &Context,
		cctx: &CommandCtx<'_>,
		cmd: &Command,
		fw: &CommandFramework,
	) -> Result<()> {
		let _ = (ctx, cctx, cmd, fw);
		this.n.fetch_add(1, Ordering::Relaxed);
		Ok(())
	}

	#[command]
	#[sub_cmds(ping, echo, outer, grp, counter, refcmd)]
	#[group]
	#[root]
	struct TestRoot;

	fn fw() -> CommandFramework {
		CommandFramework::new(&TEST_ROOT_CMD, BotConfig::default()).unwrap()
	}

	#[test]
	fn resolves_top_level_leaf_command() {
		let fw = fw();
		let (cmd, _) = fw
			.find_matching_command("ping")
			.unwrap();
		assert_eq!(cmd.names[0], "ping");
	}

	#[test]
	fn resolves_command_with_args() {
		let fw = fw();
		let (cmd, matches) = fw
			.find_matching_command("echo --text hi")
			.unwrap();
		assert_eq!(cmd.names[0], "echo");
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.text, "hi");
		// the default kicks in when the flag is omitted
		assert_eq!(args.count, 1);
	}

	#[test]
	fn resolves_explicit_arg_value() {
		let fw = fw();
		let (_, matches) = fw
			.find_matching_command("echo --text hi --count 3")
			.unwrap();
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.count, 3);
	}

	#[test]
	fn shlex_handles_quoted_args() {
		let fw = fw();
		let (_, matches) = fw
			.find_matching_command("echo --text \"hello world\"")
			.unwrap();
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.text, "hello world");
	}

	#[test]
	fn resolves_subcommand_under_group() {
		let fw = fw();
		let (cmd, matches) = fw
			.find_matching_command("outer nested echo --text x")
			.unwrap();
		assert_eq!(cmd.names[0], "echo");
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.text, "x");
	}

	#[test]
	fn resolves_to_group_when_no_leaf() {
		let fw = fw();
		let (cmd, _) = fw
			.find_matching_command("outer nested")
			.unwrap();
		assert_eq!(cmd.names[0], "nested");
		assert!(cmd.flags.contains(CommandFlags::GROUP));
	}

	#[test]
	fn unknown_command_errors() {
		let fw = fw();
		assert!(
			fw.find_matching_command("does_not_exist")
				.is_err()
		);
	}

	#[test]
	fn missing_required_arg_errors() {
		let fw = fw();
		assert!(
			fw.find_matching_command("echo")
				.is_err()
		);
	}

	#[test]
	fn tokenize_failure_errors() {
		let fw = fw();
		let Err(err) = fw.find_matching_command("echo --text \"unclosed")
		else {
			panic!("expected a tokenization error");
		};
		assert!(
			format!("{err:#}").contains("Failed to tokenize message content")
		);
	}

	#[test]
	fn unknown_flag_errors() {
		let fw = fw();
		assert!(
			fw.find_matching_command("echo --nope")
				.is_err()
		);
	}

	#[test]
	fn callable_group_runs_with_own_args() {
		let fw = fw();
		// no matching subcommand -> the group resolves to itself and its own
		// args are parsed
		let (cmd, matches) = fw
			.find_matching_command("grp --text hi")
			.unwrap();
		assert_eq!(cmd.names[0], "grp");
		assert!(cmd.flags.contains(CommandFlags::GROUP));
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.text, "hi");
	}

	#[test]
	fn callable_group_dispatches_subcommand_without_parent_args() {
		let fw = fw();
		// invoking the subcommand does not require the group's own required
		// args
		let (cmd, _) = fw
			.find_matching_command("grp ping")
			.unwrap();
		assert_eq!(cmd.names[0], "ping");
	}

	#[test]
	fn group_args_and_subcommand_are_mutually_exclusive() {
		let fw = fw();
		assert!(
			fw.find_matching_command("grp --text hi ping")
				.is_err()
		);
	}

	#[test]
	fn user_arg_uses_explicit_value() {
		let fw = fw();
		let (cmd, matches) = fw
			.find_matching_command("refcmd 123")
			.unwrap();
		assert_eq!(cmd.names[0], "refcmd");
		let args = RefArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.user, UserArg(UserId::new(123)));
	}

	#[test]
	fn user_arg_falls_back_to_referenced_user() {
		let fw = fw();
		let id = UserId::new(999);
		// the `FROM_REPLY` default is value-parsed during
		// `find_matching_command`, so the task-local must be set around it
		let (_, matches) = REFERENCED_USER
			.sync_scope(Some(id), || fw.find_matching_command("refcmd"))
			.unwrap();
		let args = RefArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.user, UserArg(id));
	}

	#[test]
	fn user_arg_errors_without_value_or_referenced_user() {
		let fw = fw();
		let res = REFERENCED_USER
			.sync_scope(None, || fw.find_matching_command("refcmd"));
		assert!(res.is_err());
	}

	#[test]
	fn stateful_group_resolves_to_itself_and_child() {
		let fw = fw();
		let (cmd, _) = fw
			.find_matching_command("counter")
			.unwrap();
		assert_eq!(cmd.names[0], "counter");

		let (child, _) = fw
			.find_matching_command("counter ping")
			.unwrap();
		assert_eq!(child.names[0], "ping");
	}
}

mod slash {
	use super::*;
	use std::collections::HashMap;

	use anyhow::Result;
	use clap::{FromArgMatches, Parser, ValueEnum};
	use macros::{SlashArgs, SlashChoices, command};
	use serenity::all::{Context, ResolvedValue};

	use crate::{
		fw::{
			Availability,
			CommandCtx,
			CommandFramework,
			slash::render_arg_tokens,
		}, util::UserArg,
	};

	#[derive(Parser)]
	struct EchoArgs {
		#[arg(long)]
		text: String,
		#[arg(long, default_value_t = 1)]
		count: u8,
	}

	#[derive(Parser)]
	struct GreetArgs {
		who: String,
	}

	/// Echo the given text.
	#[command]
	#[arg_parser = EchoArgs]
	async fn echo(
		_a: EchoArgs,
		_c: &Context,
		_x: &CommandCtx<'_>,
	) -> Result<()> {
		Ok(())
	}

	/// Greet the given user.
	#[command]
	#[arg_parser = GreetArgs]
	async fn greet(
		_a: GreetArgs,
		_c: &Context,
		_x: &CommandCtx<'_>,
	) -> Result<()> {
		Ok(())
	}

	/// Only reachable via slash.
	#[command]
	#[slash_only]
	async fn slashonly(_c: &Context, _x: &CommandCtx<'_>) -> Result<()> {
		Ok(())
	}

	/// Only reachable via prefix text.
	#[command]
	#[prefix_only]
	async fn prefixonly(_c: &Context, _x: &CommandCtx<'_>) -> Result<()> {
		Ok(())
	}

	#[derive(Parser, SlashArgs)]
	struct NativeArgs {
		who: UserArg,
	}

	/// Reference a user with a native picker.
	#[command]
	#[arg_parser = NativeArgs]
	#[slash_args]
	async fn native(
		_a: NativeArgs,
		_c: &Context,
		_x: &CommandCtx<'_>,
	) -> Result<()> {
		Ok(())
	}

	#[derive(ValueEnum, SlashChoices, Clone, Copy)]
	enum PickColor {
		Red,
		Green,
		Blue,
	}

	#[derive(Parser, SlashArgs)]
	struct ChoiceArgs {
		#[arg(long)]
		color: PickColor,
	}

	/// Pick a color from a fixed choice list.
	#[command]
	#[arg_parser = ChoiceArgs]
	#[slash_args]
	async fn choice(
		_a: ChoiceArgs,
		_c: &Context,
		_x: &CommandCtx<'_>,
	) -> Result<()> {
		Ok(())
	}

	#[derive(Parser)]
	struct ReorderArgs {
		#[arg(long)]
		opt: Option<String>,
		required: String,
	}

	/// Declares an optional option before a required one.
	#[command]
	#[arg_parser = ReorderArgs]
	async fn reorder(
		_a: ReorderArgs,
		_c: &Context,
		_x: &CommandCtx<'_>,
	) -> Result<()> {
		Ok(())
	}

	#[command]
	#[sub_cmds(echo, greet, slashonly, prefixonly, native, choice, reorder)]
	#[group]
	#[root]
	struct SlashRoot;

	fn fw() -> CommandFramework {
		CommandFramework::new(&SLASH_ROOT_CMD, BotConfig::default()).unwrap()
	}

	fn json_names(cmds: &[serenity::all::CreateCommand]) -> Vec<String> {
		serde_json::to_value(cmds)
			.unwrap()
			.as_array()
			.unwrap()
			.iter()
			.map(|c| c["name"].as_str().unwrap().to_owned())
			.collect()
	}

	#[test]
	fn availability_defaults_and_flags() {
		assert_eq!(ECHO_CMD.availability, Availability::all());
		assert_eq!(SLASHONLY_CMD.availability, Availability::SLASH);
		assert_eq!(PREFIXONLY_CMD.availability, Availability::PREFIX);
	}

	#[test]
	fn build_excludes_prefix_only_commands() {
		let names = json_names(&fw().build_slash_commands());
		assert!(names.contains(&"echo".to_owned()));
		assert!(names.contains(&"greet".to_owned()));
		assert!(names.contains(&"slashonly".to_owned()));
		// prefix-only commands are not registered as slash commands
		assert!(!names.contains(&"prefixonly".to_owned()));
	}

	#[test]
	fn schema_maps_clap_args_to_string_options() {
		let cmds = fw().build_slash_commands();
		let json = serde_json::to_value(&cmds).unwrap();
		let echo = json
			.as_array()
			.unwrap()
			.iter()
			.find(|c| c["name"] == "echo")
			.unwrap();
		// the description is captured from the handler's `///` doc comment
		assert_eq!(echo["description"], "Echo the given text.");
		let options = echo["options"].as_array().unwrap();
		let text = options
			.iter()
			.find(|o| o["name"] == "text")
			.unwrap();
		// default String-reuse: type 3 == String; `text` has no default so it
		// is required
		assert_eq!(text["type"], 3);
		assert_eq!(text["required"], true);
		let count = options
			.iter()
			.find(|o| o["name"] == "count")
			.unwrap();
		// `count` has a default, so it is optional
		assert_eq!(count["required"], false);
	}

	#[test]
	fn lowers_long_flag_options_to_tokens() {
		let clap_cmd = ECHO_CMD.parser.get(&ECHO_CMD);
		let mut provided = HashMap::new();
		provided.insert("text", ResolvedValue::String("hi"));
		let tokens = render_arg_tokens(clap_cmd, &provided).unwrap();
		assert_eq!(tokens, vec!["--text".to_owned(), "hi".to_owned()]);

		// the synthesized tokens must round-trip back through the normal
		// command pipeline
		let fw = fw();
		let full = std::iter::once("echo".to_owned())
			.chain(tokens)
			.collect::<Vec<_>>();
		let (cmd, matches) = fw.match_command_tokens(full).unwrap();
		assert_eq!(cmd.names[0], "echo");
		let args = EchoArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.text, "hi");
		assert_eq!(args.count, 1);
	}

	#[test]
	fn lowers_positional_options_to_tokens() {
		let clap_cmd = GREET_CMD.parser.get(&GREET_CMD);
		let mut provided = HashMap::new();
		provided.insert("who", ResolvedValue::String("bob"));
		let tokens = render_arg_tokens(clap_cmd, &provided).unwrap();
		// positional args are emitted as bare values, not `--who bob`
		assert_eq!(tokens, vec!["bob".to_owned()]);

		let fw = fw();
		let (cmd, matches) = fw
			.match_command_tokens(vec!["greet".to_owned(), "bob".to_owned()])
			.unwrap();
		assert_eq!(cmd.names[0], "greet");
		let args = GreetArgs::from_arg_matches(&matches).unwrap();
		assert_eq!(args.who, "bob");
	}

	#[test]
	fn native_typing_overrides_option_kind() {
		let cmds = fw().build_slash_commands();
		let json = serde_json::to_value(&cmds).unwrap();
		let native = json
			.as_array()
			.unwrap()
			.iter()
			.find(|c| c["name"] == "native")
			.unwrap();
		let who = native["options"]
			.as_array()
			.unwrap()
			.iter()
			.find(|o| o["name"] == "who")
			.unwrap();
		// `#[slash_args]` promotes the `UserArg` field to a native User picker
		// (type 6) instead of the default String (type 3)
		assert_eq!(who["type"], 6);
	}

	#[test]
	fn value_enum_registers_string_choices() {
		let cmds = fw().build_slash_commands();
		let json = serde_json::to_value(&cmds).unwrap();
		let choice = json
			.as_array()
			.unwrap()
			.iter()
			.find(|c| c["name"] == "choice")
			.unwrap();
		let color = choice["options"]
			.as_array()
			.unwrap()
			.iter()
			.find(|o| o["name"] == "color")
			.unwrap();
		// a `ValueEnum` maps to a String option (type 3) carrying its variants
		// as fixed choices
		assert_eq!(color["type"], 3);
		let choices = color["choices"].as_array().unwrap();
		let values: Vec<&str> = choices
			.iter()
			.map(|c| c["value"].as_str().unwrap())
			.collect();
		assert_eq!(values, vec!["red", "green", "blue"]);
	}

	#[test]
	fn required_options_are_ordered_before_optional() {
		let cmds = fw().build_slash_commands();
		let json = serde_json::to_value(&cmds).unwrap();
		let reorder = json
			.as_array()
			.unwrap()
			.iter()
			.find(|c| c["name"] == "reorder")
			.unwrap();
		let options = reorder["options"].as_array().unwrap();
		// Discord requires required options first, so `required` is emitted
		// before the optional `opt` despite the reverse declaration order
		assert_eq!(options[0]["name"], "required");
		assert_eq!(options[0]["required"], true);
		assert_eq!(options[1]["name"], "opt");
		assert_eq!(options[1]["required"], false);
	}

	#[test]
	fn resolve_node_walks_the_tree() {
		let fw = fw();
		let node = fw
			.resolve_node(&["echo".to_owned()])
			.unwrap();
		assert_eq!(node.names[0], "echo");
		assert!(
			fw.resolve_node(&["nope".to_owned()])
				.is_none()
		);
	}
}
