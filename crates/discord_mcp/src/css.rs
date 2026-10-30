//! Pretty printing and indexing Discord's minified css
//!
//! The formatter only re-indents: every byte of the original rules and
//! declarations is kept, so what a search finds is exactly what the client
//! ships.

use std::{collections::HashMap, ops::Range};

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use smol_str::SmolStr;

use crate::text::LineIndex;

const INDENT: &str = "  ";

/// A `{ ... }` block: a style rule, or an at-rule like `@media`
pub struct CssBlock {
	/// the selector, or the at-rule and its params
	pub prelude: String,
	/// the preludes of the at-rules this block is nested in, outermost first
	pub context: Vec<String>,
	/// from the start of the prelude to the end of the closing brace, in
	/// [`CssChunk::code`]
	pub range: Range<usize>,
	/// the text between the braces
	pub body: Range<usize>,
	/// whether other blocks are nested in this one
	pub has_children: bool,
}

pub struct CssChunk {
	pub name: SmolStr,
	/// the formatted css
	pub code: String,
	pub raw_len: usize,
	pub lines: LineIndex,
	/// sorted by start
	pub blocks: Vec<CssBlock>,
}

impl CssChunk {
	pub fn new(name: SmolStr, raw: &str) -> Self {
		let (code, mut blocks) = format_css(raw);
		blocks.sort_unstable_by_key(|b| b.range.start);
		Self {
			name,
			lines: LineIndex::new(&code),
			code,
			raw_len: raw.len(),
			blocks,
		}
	}

	/// The innermost block containing byte `pos` of [`Self::code`]
	pub fn block_at(&self, pos: usize) -> Option<usize> {
		let candidates = self
			.blocks
			.partition_point(|b| b.range.start <= pos);
		// the closest block starting before pos that also contains it is the
		// innermost one, since blocks nest
		(0..candidates)
			.rev()
			.find(|&i| self.blocks[i].range.contains(&pos))
	}

	pub fn block_line(&self, block: &CssBlock) -> usize {
		self.lines.line(block.range.start)
	}
}

/// A custom property, eg `--background-primary`
#[derive(Default)]
pub struct CssVar {
	/// `(chunk, block, value)` for every declaration of the variable
	pub defs: Vec<(usize, usize, String)>,
	/// how many times `var(--name` appears
	pub uses: usize,
}

pub struct CssIndex {
	/// sorted by name
	pub chunks: Vec<CssChunk>,
	pub vars: HashMap<SmolStr, CssVar>,
}

impl CssIndex {
	pub fn new(raw: &HashMap<SmolStr, String>) -> Self {
		let mut chunks = raw
			.iter()
			.collect::<Vec<_>>()
			.into_par_iter()
			.map(|(name, css)| CssChunk::new(name.clone(), css))
			.collect::<Vec<_>>();
		chunks.sort_unstable_by(|a, b| a.name.cmp(&b.name));
		let vars = collect_vars(&chunks);
		Self { chunks, vars }
	}

	/// Finds a chunk by exact name, or a unique name prefix
	pub fn chunk(&self, name: &str) -> Result<usize, String> {
		let name = name.trim();
		if let Some(i) = self
			.chunks
			.iter()
			.position(|c| c.name == name)
		{
			return Ok(i);
		}
		let matches = self
			.chunks
			.iter()
			.enumerate()
			.filter(|(_, c)| c.name.starts_with(name))
			.map(|(i, _)| i)
			.collect::<Vec<_>>();
		match matches.as_slice() {
			[i] => Ok(*i),
			[] => Err(format!("No css chunk named {name:?}")),
			_ => Err(format!("{name:?} matches {} css chunks", matches.len())),
		}
	}
}

fn collect_vars(chunks: &[CssChunk]) -> HashMap<SmolStr, CssVar> {
	let mut vars = HashMap::<SmolStr, CssVar>::new();
	for (ci, chunk) in chunks.iter().enumerate() {
		for (bi, block) in chunk.blocks.iter().enumerate() {
			if block.has_children {
				continue;
			}
			for decl in chunk.code[block.body.clone()].lines() {
				let decl = decl.trim().trim_end_matches(';');
				if let Some(rest) = decl.strip_prefix("--")
					&& let Some((name, value)) = rest.split_once(':')
				{
					vars.entry(SmolStr::from(format!("--{}", name.trim())))
						.or_default()
						.defs
						.push((ci, bi, value.trim().to_owned()));
				}
			}
		}
		let mut rest = chunk.code.as_str();
		while let Some(at) = rest.find("var(") {
			rest = &rest[at + 4..];
			let name = rest
				.trim_start()
				.split(|c: char| c == ',' || c == ')' || c.is_whitespace())
				.next()
				.unwrap_or_default();
			if name.starts_with("--") {
				vars.entry(SmolStr::from(name))
					.or_default()
					.uses += 1;
			}
		}
	}
	vars
}

struct Open {
	prelude: String,
	start: usize,
	body_start: usize,
	has_children: bool,
}

struct Formatter {
	out: String,
	pending: String,
	stack: Vec<Open>,
	blocks: Vec<CssBlock>,
}

impl Formatter {
	fn indent(&mut self) {
		for _ in 0..self.stack.len() {
			self.out.push_str(INDENT);
		}
	}

	fn take_pending(&mut self) -> String {
		let text = self.pending.trim().to_owned();
		self.pending.clear();
		text
	}

	fn push_space(&mut self) {
		if !self.pending.is_empty() && !self.pending.ends_with(' ') {
			self.pending.push(' ');
		}
	}

	fn open(&mut self) {
		let prelude = self.take_pending();
		if let Some(parent) = self.stack.last_mut() {
			parent.has_children = true;
		}
		let start = self.out.len();
		self.indent();
		self.out.push_str(&prelude);
		self.out.push_str(" {\n");
		self.stack.push(Open {
			prelude,
			start,
			body_start: self.out.len(),
			has_children: false,
		});
	}

	fn statement(&mut self, end: &str) {
		let text = self.take_pending();
		if !text.is_empty() {
			self.indent();
			self.out.push_str(&text);
			self.out.push_str(end);
			self.out.push('\n');
		}
	}

	fn close(&mut self) {
		// the last declaration of a block usually has no `;`
		self.statement("");
		let Some(open) = self.stack.pop() else {
			// unbalanced, drop it
			return;
		};
		let body = open.body_start..self.out.len();
		self.indent();
		self.out.push_str("}\n");
		self.blocks.push(CssBlock {
			context: self
				.stack
				.iter()
				.map(|o| o.prelude.clone())
				.collect(),
			prelude: open.prelude,
			range: open.start..self.out.len(),
			body,
			has_children: open.has_children,
		});
	}
}

/// Re-indents `src`, one declaration per line, returning the formatted css
/// and every block in it
pub fn format_css(src: &str) -> (String, Vec<CssBlock>) {
	let mut f = Formatter {
		out: String::with_capacity(src.len() + src.len() / 4),
		pending: String::new(),
		stack: Vec::new(),
		blocks: Vec::new(),
	};
	let mut parens = 0usize;
	let mut chars = src.char_indices().peekable();
	while let Some((i, c)) = chars.next() {
		match c {
			'"' | '\'' => {
				f.pending.push(c);
				while let Some((_, s)) = chars.next() {
					f.pending.push(s);
					if s == '\\' {
						if let Some((_, escaped)) = chars.next() {
							f.pending.push(escaped);
						}
					} else if s == c {
						break;
					}
				}
			}
			'/' if src[i..].starts_with("/*") => {
				let end = src[i + 2..]
					.find("*/")
					.map_or(src.len(), |e| i + 2 + e + 2);
				f.pending.push_str(&src[i..end]);
				while chars
					.peek()
					.is_some_and(|&(j, _)| j < end)
				{
					chars.next();
				}
			}
			'(' => {
				parens += 1;
				f.pending.push(c);
			}
			')' => {
				parens = parens.saturating_sub(1);
				f.pending.push(c);
			}
			'{' if parens == 0 => f.open(),
			'}' if parens == 0 => f.close(),
			';' if parens == 0 => f.statement(";"),
			c if c.is_whitespace() => f.push_space(),
			c => f.pending.push(c),
		}
	}
	f.statement("");
	while !f.stack.is_empty() {
		f.close();
	}
	(f.out, f.blocks)
}

#[cfg(test)]
mod tests {
	use super::*;

	const CSS: &str = r#".a_1{color:red;background:url("data:a;b{c}")}@media (max-width:10px){.b_2 .c{--x:1px;margin:0 var(--y,2px)}}:root{--y:3px}"#;

	#[test]
	fn formats_and_finds_blocks() {
		let (code, blocks) = format_css(CSS);
		assert_eq!(
			code,
			".a_1 {\n  color:red;\n  background:url(\"data:a;b{c}\")\n}\n\
			 @media (max-width:10px) {\n  .b_2 .c {\n    --x:1px;\n    \
			 margin:0 var(--y,2px)\n  }\n}\n:root {\n  --y:3px\n}\n"
		);
		let preludes = blocks
			.iter()
			.map(|b| b.prelude.as_str())
			.collect::<Vec<_>>();
		assert_eq!(
			preludes,
			[".a_1", ".b_2 .c", "@media (max-width:10px)", ":root"]
		);
		let nested = &blocks[1];
		assert_eq!(nested.context, ["@media (max-width:10px)"]);
		assert!(blocks[2].has_children);
	}

	#[test]
	fn innermost_block() {
		let chunk = CssChunk::new("x.css".into(), CSS);
		let pos = chunk.code.find("margin").unwrap();
		let block = &chunk.blocks[chunk.block_at(pos).unwrap()];
		assert_eq!(block.prelude, ".b_2 .c");
		assert_eq!(chunk.block_line(block), 6);
	}

	#[test]
	fn collects_vars() {
		let mut raw = HashMap::new();
		raw.insert(SmolStr::from("x.css"), CSS.to_owned());
		let index = CssIndex::new(&raw);
		assert_eq!(index.vars["--y"].uses, 1);
		assert_eq!(index.vars["--y"].defs[0].2, "3px");
		assert_eq!(index.vars["--x"].defs.len(), 1);
	}

	#[test]
	fn survives_unbalanced_input() {
		let (code, blocks) = format_css("}.a{color:red");
		assert_eq!(code, ".a {\n  color:red\n}\n");
		assert_eq!(blocks.len(), 1);
	}
}
