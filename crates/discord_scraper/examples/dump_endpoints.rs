//! Scrapes the current Discord build, finds the endpoints module with
//! [`as_endpoints_module`](webpack_ast_parser::WebpackAstParser::as_endpoints_module)
//! and prints a `.d.ts` describing it to stdout.
//!
//! Every endpoint function gets one type param per argument, so calls resolve
//! to the exact path, eg: `Endpoints.USER("123")` is typed as `"/users/123"`.
//!
//! ```sh
//! cargo run -p discord_scraper --example dump_endpoints [stable|canary] > endpoints.d.ts
//! ```
use std::{
	env,
	fmt::{self, Display, Formatter},
	io::{self, Write as _},
	sync::Arc,
};

use anyhow::{Context, Result, bail};
use discord_scraper::{BarProgress, JsScraper, make_reqwest_client};
use explorer_types::{Channel, ModuleId};
use miette_ui::install_miette_hook;
use oxc_allocator::Allocator;
use term_progress::MultiProgressWrapper;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use webpack_ast_parser::{
	WebpackAstParser,
	endpoints::{Endpoint, EndpointParam, Segment},
};

const INDENT: u8 = 4;

/// Strings every endpoints module contains. Used to avoid formatting and
/// parsing every module in the bundle
const MARKERS: [&str; 3] =
	["GUILD_CHANNELS", "GUILD_MEMBER_NICK", "MESSAGE_ACK"];

fn install_tracing(bars: MultiProgressWrapper) {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info"));
	tracing_subscriber::fmt()
		.with_env_filter(filter)
		.with_writer(move || bars.writer())
		.with_ansi_sanitization(false)
		.init();
}

fn parse_channel() -> Result<Channel> {
	match env::args().nth(1).as_deref() {
		None | Some("stable") => Ok(Channel::Stable),
		Some("canary") => Ok(Channel::Canary),
		Some(other) => bail!("Unknown channel {other:?}"),
	}
}

/// Adds the module header and pretty-prints `src`, falling back to the
/// unformatted module if formatting fails
fn format_module(id: ModuleId, src: &mut String) {
	WebpackAstParser::format_module_header(src, id, false);
	match pretty_printer::format_to_str(src, INDENT) {
		Ok(formatted) => *src = formatted,
		Err(e) => warn!(%id, "Failed to format module, using it as-is: {e}"),
	}
}

/// Parses `src` as an endpoints module, logging any errors
fn parse_endpoints(id: ModuleId, src: &str) -> Option<Vec<Endpoint>> {
	let name = format!("{id}.js");
	let alloc = Allocator::default();
	let parser = match WebpackAstParser::try_new(&alloc, src) {
		Ok(parser) => parser,
		Err(e) => {
			warn!(
				%id,
				"Failed to parse module: {:?}",
				e.with_local_source(src, &name)
			);
			return None;
		}
	};
	match parser.as_endpoints_module() {
		None => {
			debug!(%id, "Not an endpoints module");
			None
		}
		Some(Ok(endpoints)) => Some(endpoints),
		Some(Err(e)) => {
			warn!(
				%id,
				"Failed to parse endpoints: {:?}",
				e.with_local_source(src, &name)
			);
			None
		}
	}
}

/// Writes `s` as a double quoted string literal
fn write_str_lit(f: &mut Formatter<'_>, s: &str) -> fmt::Result {
	f.write_str(&serde_json::to_string(s).map_err(|_| fmt::Error)?)
}

fn is_identifier(s: &str) -> bool {
	let mut chars = s.chars();
	chars
		.next()
		.is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
		&& chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The TS type of the string produced by a list of segments
struct SegmentsType<'a>(&'a [Segment]);

impl Display for SegmentsType<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self.0 {
			[] => return f.write_str("\"\""),
			[Segment::Literal { value }] => return write_str_lit(f, value),
			_ => {}
		}
		f.write_str("`")?;
		for seg in self.0 {
			match seg {
				Segment::Literal { value } => {
					for c in value.chars() {
						if matches!(c, '\\' | '`' | '$') {
							f.write_str("\\")?;
						}
						write!(f, "{c}")?;
					}
				}
				// `escape` can change the param, but the return type is only
				// a hint, so it's treated as unescaped
				Segment::Param { index } | Segment::EscapedParam { index } => {
					write!(f, "${{P{index}}}")?;
				}
				// globals are only known at runtime
				Segment::Global { .. } => {
					f.write_str("${string}")?;
				}
				Segment::Conditional {
					param,
					consequent,
					alternate,
				} => write!(
					f,
					"${{P{param} extends null | undefined ? {} : {}}}",
					SegmentsType(alternate),
					SegmentsType(consequent),
				)?,
			}
		}
		f.write_str("`")
	}
}

/// Marks every param that is checked for nullishness in `segments`
fn mark_nullable(segments: &[Segment], nullable: &mut [bool]) {
	for seg in segments {
		if let Segment::Conditional {
			param,
			consequent,
			alternate,
		} = seg
		{
			nullable[*param] = true;
			mark_nullable(consequent, nullable);
			mark_nullable(alternate, nullable);
		}
	}
}

/// One property of the `Endpoints` object type
struct EndpointDecl<'a>(&'a Endpoint);

impl Display for EndpointDecl<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let Endpoint {
			name,
			value,
			params,
			is_function,
		} = self.0;

		let defaults = params
			.iter()
			.enumerate()
			.filter_map(|(i, param)| match param {
				EndpointParam::Optional { default } => Some((i, default)),
				EndpointParam::Required => None,
			})
			.collect::<Vec<_>>();
		if !defaults.is_empty() {
			f.write_str("    /**\n")?;
			for (i, default) in defaults {
				writeln!(f, "     * @param arg{i} defaults to `{default}`")?;
			}
			f.write_str("     */\n")?;
		}

		f.write_str("    readonly ")?;
		if is_identifier(name) {
			f.write_str(name)?;
		} else {
			write_str_lit(f, name)?;
		}
		f.write_str(": ")?;
		if !is_function {
			return writeln!(f, "{};", SegmentsType(value));
		}

		let mut nullable = vec![false; params.len()];
		mark_nullable(value, &mut nullable);
		// only trailing params can be omitted
		let first_omittable = params
			.iter()
			.zip(&nullable)
			.rposition(|(param, nullable)| {
				matches!(param, EndpointParam::Required) && !nullable
			})
			.map_or(0, |i| i + 1);

		if !params.is_empty() {
			f.write_str("<")?;
			for (i, param) in params.iter().enumerate() {
				if i != 0 {
					f.write_str(", ")?;
				}
				write!(f, "P{i} extends Param")?;
				if nullable[i] {
					f.write_str(" | null | undefined")?;
				}
				if i >= first_omittable {
					// an omitted optional param is replaced with its default,
					// which we can't know the type of
					f.write_str(match param {
						EndpointParam::Optional { .. } => " = string",
						EndpointParam::Required => " = undefined",
					})?;
				}
			}
			f.write_str(">")?;
		}
		f.write_str("(")?;
		for i in 0..params.len() {
			if i != 0 {
				f.write_str(", ")?;
			}
			let opt = if i >= first_omittable { "?" } else { "" };
			write!(f, "arg{i}{opt}: P{i}")?;
		}
		writeln!(f, ") => {};", SegmentsType(value))
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let bars = MultiProgressWrapper::default();
	install_tracing(bars.clone());
	install_miette_hook(true);
	let channel = parse_channel()?;

	let client = make_reqwest_client()?;
	let res = client
		.get(channel.app_base())
		.send()
		.await?;
	let build_hash = res
		.headers()
		.get("x-build-id")
		.context("Response did not include build hash header")?
		.to_str()?
		.to_owned();
	info!("{channel:?} build hash: {build_hash}");
	let html = res.text().await?;

	let progress = Arc::new(BarProgress::new(bars, channel));
	let scraped = JsScraper::scrape(&html, channel, client, progress).await?;
	info!(
		"scraped {} modules from build {}",
		scraped.modules.len(),
		scraped.build_number
	);

	let mut candidates = scraped
		.modules
		.into_iter()
		.filter(|(_, src)| MARKERS.iter().all(|m| src.contains(m)))
		.collect::<Vec<_>>();
	candidates.sort_unstable_by_key(|(id, _)| *id);
	info!("found {} candidate modules", candidates.len());

	let mut found = None;
	for (id, mut src) in candidates {
		format_module(id, &mut src);
		let Some(endpoints) = parse_endpoints(id, &src) else {
			continue;
		};
		info!(%id, "found {} endpoints", endpoints.len());
		if found.is_some() {
			warn!(%id, "found multiple endpoints modules, ignoring this one");
			continue;
		}
		found = Some((id, endpoints));
	}
	let (id, endpoints) = found.context("Failed to find endpoints module")?;

	let mut out = io::stdout().lock();
	writeln!(
		out,
		"// Generated by `cargo run -p discord_scraper --example \
		 dump_endpoints`\n// from Discord {channel:?} build {} \
		 ({build_hash}), module {id}\n",
		scraped.build_number
	)?;
	writeln!(out, "export type Param = string | number;\n")?;
	writeln!(out, "export interface Endpoints {{")?;
	for endpoint in &endpoints {
		write!(out, "{}", EndpointDecl(endpoint))?;
	}
	writeln!(out, "}}")?;
	out.flush()?;
	Ok(())
}
