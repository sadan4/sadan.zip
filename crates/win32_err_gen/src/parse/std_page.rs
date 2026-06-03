use anyhow::{Context as _, Result, anyhow, bail};
use markdown::mdast::{self, Node, Paragraph, Text};
use tracing::warn;

use crate::types::W32Error;

fn is_code_node(node: &Node) -> bool {
	let Node::Paragraph(node) = node else {
		return false;
	};
	let Some(Node::Html(node)) = node.children.first() else {
		return false;
	};
	node.value.starts_with("<span id=")
}

fn parse_code_node(node: &Node) -> Result<String> {
	let Node::Paragraph(Paragraph {
		children: name_html_tags,
		..
	}) = node
	else {
		bail!("expected paragraph node");
	};
	let Some(Node::Strong(name_node)) = name_html_tags.last() else {
		bail!("Failed to find name node");
	};
	let Some(Node::Text(Text { value: message, .. })) =
		name_node.children.first()
	else {
		bail!("Failed to find name text node");
	};
	Ok(message.clone())
}

fn parse_id_node(node: &Node) -> Result<u32> {
	let Node::Paragraph(Paragraph {
		children: id_text, ..
	}) = node
	else {
		bail!("expected paragraph node");
	};
	let Some(Node::Text(Text {
		value: unparsed_id, ..
	})) = id_text.first()
	else {
		bail!("Failed to find id text node");
	};
	let Some(id) = unparsed_id
		.bytes()
		.position(|b| b == b' ')
	else {
		bail!("Failed to find dec/hex separator(' ')");
	};
	let unparsed_id = &unparsed_id[..id];
	let code = match unparsed_id.parse::<u32>() {
		Ok(code) => code,
		Err(err) => {
			bail!("Failed to parse id as decimal: {err}");
		}
	};
	Ok(code)
}

fn parse_desc_node(node: &Node, ctx: &mut String) -> Result<()> {
	let Node::Paragraph(Paragraph { children, .. }) = node else {
		bail!("Expected paragraph node");
	};
	for c in children {
		match c {
			Node::Text(Text { value, .. }) => ctx.push_str(value),
			_ => {
				warn!("Unexpected node type in description: {c:?}");
			}
		}
	}
	ctx.push('\n');
	Ok(())
}

pub fn parse(node: &mdast::Node) -> Result<Vec<W32Error>> {
	let Node::Root(root) = node else {
		bail!("expected root node");
	};
	let nodes = root.children.as_slice();
	let start_idx = nodes
		.iter()
		.position(is_code_node)
		.context("Failed to find starting node")?;
	let nodes = &nodes[start_idx..];
	let end_idx = nodes
		.iter()
		.rev()
		.position(|node| {
			let Node::Heading(node) = node else {
				return false;
			};
			if node.depth != 2 || node.children.len() != 1 {
				return false;
			}
			let Node::Text(node) = node.children.first().unwrap() else {
				return false;
			};
			node.value == "Requirements"
		})
		.map(|idx| nodes.len() - idx)
		.context("Failed to find ending node")?;
	let nodes = &nodes[..end_idx - 1];
	let mut ret = Vec::with_capacity(nodes.len() / 3);
	let chunks = nodes.chunk_by(|_, node| !is_code_node(node));
	'outer: for chunk in chunks {
		let mut nodes = chunk.iter();
		let message = match nodes
			.next()
			.ok_or_else(|| anyhow!("expected node"))
			.and_then(parse_code_node)
		{
			Ok(code) => code,
			Err(err) => {
				warn!("Failed to parse code node: {err:#?}");
				continue;
			}
		};
		let code = match nodes
			.next()
			.ok_or_else(|| anyhow!("expected node"))
			.and_then(parse_id_node)
		{
			Ok(code) => code,
			Err(err) => {
				warn!("Failed to parse id node: {err:#?}");
				continue;
			}
		};
		let mut desc = String::new();
		for node in nodes {
			if let Err(err) = parse_desc_node(node, &mut desc) {
				warn!("Failed to parse desc node: {err:?}");
				continue 'outer;
			}
		}
		// pop trailing newline added by parse_desc_node
		let c = desc.pop();
		debug_assert_eq!(c, Some('\n'));
		ret.push(W32Error {
			code,
			message: message.clone(),
			desc,
		});
	}
	Ok(ret)
}
