use super::*;

#[test]
fn cuts_plain_text_at_a_codepoint_boundary() {
	let s = "héllo";
	// the budget counts codepoints, not the bytes the é takes up
	assert_eq!(ansi_truncation_point(s, 5), s.len());
	assert_eq!(ansi_truncation_point(s, 9), s.len());
	assert_eq!(&s[..ansi_truncation_point(s, 2)], "hé");
}

#[test]
fn never_cuts_inside_a_control_sequence() {
	let s = "ab\u{1b}[1;31mcd";
	// every cut that would land inside the sequence falls back to its
	// start
	for max in 2..=8 {
		assert_eq!(&s[..ansi_truncation_point(s, max)], "ab");
	}
	assert_eq!(&s[..ansi_truncation_point(s, 9)], "ab\u{1b}[1;31m");
}

#[test]
fn never_cuts_inside_a_two_character_escape() {
	let s = "a\u{1b}Mb";
	assert_eq!(&s[..ansi_truncation_point(s, 1)], "a");
	assert_eq!(&s[..ansi_truncation_point(s, 2)], "a");
	assert_eq!(&s[..ansi_truncation_point(s, 3)], "a\u{1b}M");
}

mod command_framework {
	use std::{
		future,
		sync::atomic::{AtomicU64, Ordering},
	};

	use anyhow::Result;
	use clap::{ArgMatches, FromArgMatches, Parser};
	use macros::{command, executor};
	use serenity::all::{Context, Message, UserId};

	use crate::{
		fw::{Command, CommandFlags, CommandFramework},
		util::{FROM_REPLY, REFERENCED_USER, UserArg},
	};

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
	async fn refcmd(args: RefArgs, ctx: &Context, msg: &Message) -> Result<()> {
		let _ = (args, ctx, msg);
		Ok(())
	}

	#[command]
	#[arg_parser = EchoArgs]
	async fn echo(args: EchoArgs, ctx: &Context, msg: &Message) -> Result<()> {
		let _ = (args, ctx, msg);
		Ok(())
	}

	#[command]
	async fn ping(ctx: &Context, msg: &Message) -> Result<()> {
		let _ = (ctx, msg);
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
	async fn grp(args: EchoArgs, ctx: &Context, msg: &Message) -> Result<()> {
		let _ = (args, ctx, msg);
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
		fn new(_: &CommandFramework) -> future::Ready<Self> {
			future::ready(Self {
				n: AtomicU64::new(0),
			})
		}
	}

	#[executor]
	async fn counter(
		this: &Counter,
		ctx: &Context,
		msg: &Message,
		cmd: &Command,
		fw: &CommandFramework,
		args: &ArgMatches,
	) -> Result<()> {
		let _ = (ctx, msg, cmd, fw, args);
		this.n.fetch_add(1, Ordering::Relaxed);
		Ok(())
	}

	#[command]
	#[sub_cmds(ping, echo, outer, grp, counter, refcmd)]
	#[group]
	#[root]
	struct TestRoot;

	fn fw() -> CommandFramework {
		CommandFramework::new(&TEST_ROOT_CMD)
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
