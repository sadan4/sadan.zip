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
	use anyhow::Result;
	use clap::{FromArgMatches, Parser};
	use macros::command;
	use serenity::all::{Context, Message};

	use crate::fw::{CommandFlags, CommandFramework};

	#[derive(Parser)]
	struct EchoArgs {
		#[arg(long)]
		text: String,
		#[arg(long, default_value_t = 1)]
		count: u8,
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

	#[command]
	#[sub_cmds(ping, echo, outer)]
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
}
