use anyhow::{Context as _, Result, bail};
use markdown::mdast::{
	self,
	Emphasis,
	Link,
	Node,
	Paragraph,
	Root,
	Strong,
	TableCell,
	TableRow,
	Text,
};

use crate::W32Error;

fn handle_code_msg_cell(node: Node) -> Result<(u32, String)> {
	let Node::TableCell(TableCell { children, .. }) = node else {
		bail!("Expected table cell node");
	};
	let mut children = children.into_iter();
	let code_node = children
		.next()
		.context("Failed to get code node")?;
	let mut children = children.skip(2);
	let msg_node = children
		.next()
		.context("Failed to get message node")?;
	let Node::Text(Text { value: message, .. }) = msg_node else {
		bail!("expected message node to be text node");
	};
	let Node::Text(Text { value: u_code, .. }) = code_node else {
		bail!("expected code node to be text node");
	};
	let u_code = if u_code.starts_with("0x") || u_code.starts_with("0X") {
		&u_code[2..]
	} else {
		&u_code[..]
	};
	let code = u32::from_str_radix(u_code, 16)
		.context("Failed to parse code as hex")?;
	Ok((code, message))
}

fn handle_desc_cell(node: Node, desc: &mut String) -> Result<()> {
	match node {
		Node::Paragraph(Paragraph { children, .. }) => {
			for child in children {
				handle_desc_cell(child, desc)?;
			}
			desc.push('\n');
			Ok(())
		}
		Node::Text(Text { value, .. }) => {
			desc.push_str(&value);
			Ok(())
		}
		Node::Link(Link { children, .. }) => {
			for child in children {
				handle_desc_cell(child, desc)?;
			}
			Ok(())
		}
		Node::Emphasis(Emphasis { children, .. }) => {
			desc.push('*');
			for child in children {
				handle_desc_cell(child, desc)?;
			}
			desc.push('*');
			Ok(())
		}
		Node::Strong(Strong { children, .. }) => {
			desc.push_str("**");
			for child in children {
				handle_desc_cell(child, desc)?;
			}
			desc.push_str("**");
			Ok(())
		}
		Node::TableCell(TableCell { children, .. }) if desc.is_empty() => {
			for child in children {
				handle_desc_cell(child, desc)?;
			}
			Ok(())
		}
		_ => {
			bail!("Unexpected node type in description: {node:?}");
		}
	}
}

fn handle_row(node: Node) -> Result<W32Error> {
	let Node::TableRow(TableRow {
		children: cells, ..
	}) = node
	else {
		bail!("expected table row node");
	};
	let mut cells = cells.into_iter();
	let code_message_cell = cells
		.next()
		.context("Failed to get code/message cell")?;
	let desc_cell = cells
		.next()
		.context("Failed to get description cell")?;
	let (code, message) = handle_code_msg_cell(code_message_cell)
		.context("Failed to handle code/message cell")?;
	let mut desc = String::new();
	handle_desc_cell(desc_cell, &mut desc)
		.context("Failed to handle description cell")?;

	Ok(W32Error {
		code,
		message,
		desc,
	})
}

pub fn parse(node: mdast::Node) -> Result<Vec<W32Error>> {
	let Node::Root(Root {
		children: nodes, ..
	}) = node
	else {
		bail!("expected root node");
	};
	let tbl = nodes
		.into_iter()
		.find_map(|node| {
			if let Node::Table(tbl) = node {
				Some(tbl)
			} else {
				None
			}
		})
		.context("Failed to find table node")?;
	let mut ret = Vec::with_capacity(tbl.children.len());
	// skip header row
	for row in tbl.children.into_iter().skip(1) {
		let error = match handle_row(row) {
			Ok(error) => error,
			Err(e) => {
				eprintln!("Failed to handle row: {e:?}");
				continue;
			}
		};
		ret.push(error);
	}
	Ok(ret)
}
