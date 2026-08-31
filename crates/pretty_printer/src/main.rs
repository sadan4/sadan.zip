use std::{
	env,
	fs,
	io::{self, IsTerminal as _, Write},
	path::{Path, PathBuf},
	process::exit,
	sync::atomic::{AtomicBool, Ordering},
	thread,
};

use clap::{CommandFactory as _, Parser};
use clap_complete::Shell;

use anyhow::{Context as _, Result, bail};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	/// If true, input files will be formatted in-place
	///
	/// Does nothing on stdin input
	///
	/// required if multiple files are provided
	#[arg(short, long)]
	pub in_place: bool,
	/// the indent size, in spaces
	///
	/// a value of 0 means use tabs
	#[arg(short = 'd', long, default_value_t = 0)]
	pub indent: u8,
	/// generate completions for the given shell
	#[arg(long)]
	pub completions: Option<Shell>,
	/// The files to format, if none are provided reads from stdin
	pub files: Vec<PathBuf>,
}

fn handle_stdin(indent_size: u8) -> Result<()> {
	let stdin = io::stdin();
	if stdin.is_terminal() {
		Cli::command()
			.print_long_help()
			.unwrap();
		bail!("stdin is a terminal, cant read anything");
	}
	let contents =
		io::read_to_string(stdin).context("Failed to read file from stdin")?;
	let formatted = pretty_printer::format_to_str(&contents, indent_size)
		.context("Failed to format file from stdin")?;
	io::stdout()
		.write_all(formatted.as_bytes())
		.context("Failed to write formatted file to stdout")?;
	Ok(())
}

fn handle_single_file(path: &Path, indent_size: u8) -> Result<String> {
	let contents = fs::read_to_string(path)?;
	let formatted = pretty_printer::format_to_str(&contents, indent_size)
		.with_context(|| {
			format!("Failed to format file: {}", path.display())
		})?;
	Ok(formatted)
}

fn main() {
	let cli = Cli::parse();
	if let Some(shell) = cli.completions {
		clap_complete::generate(
			shell,
			&mut Cli::command(),
			env!("CARGO_BIN_NAME"),
			&mut io::stdout(),
		);
		exit(0);
	}
	if cli.files.is_empty() || cli.files.len() == 1 && *cli.files[0] == *"-" {
		let Err(e) = handle_stdin(cli.indent) else {
			exit(0);
		};
		eprintln!("Failed to format stdin: {e:?}");
		exit(1);
	}
	if cli.files.len() == 1 {
		let path = &cli.files[0];
		match handle_single_file(path, cli.indent).and_then(|str| {
			if cli.in_place {
				fs::write(path, str).with_context(|| {
					format!("Failed to write file: {}", path.display())
				})
			} else {
				io::stdout()
					.write_all(str.as_bytes())
					.context("Failed to write to stdout")
			}
		}) {
			Ok(()) => exit(0),
			Err(e) => {
				eprintln!("Failed formatting file: {e:?}");
				exit(1);
			}
		}
	}
	if !cli.in_place {
		eprintln!("Cannot format multiple files to stdout, use --in-place");
		exit(1)
	}
	let mut handles = Vec::with_capacity(cli.files.len());
	static HAS_ERR: AtomicBool = AtomicBool::new(false);
	for path in cli.files {
		let indent = cli.indent;
		let handle = thread::spawn(move || {
			match handle_single_file(&path, indent).and_then(|str| {
				fs::write(&path, str).with_context(|| {
					format!("Failed to write file: {}", path.display())
				})
			}) {
				Ok(()) => {}
				Err(e) => {
					HAS_ERR.store(true, Ordering::Relaxed);
					eprintln!(
						"Failed formatting file {}: {e:?}",
						path.display()
					);
				}
			}
		});
		handles.push(handle);
	}
	for handle in handles {
		if let Err(e) = handle.join() {
			HAS_ERR.store(true, Ordering::Relaxed);
			eprintln!("Thread panicked. err: {e:?}");
		}
	}
	exit(HAS_ERR.load(Ordering::Relaxed).into());
}
