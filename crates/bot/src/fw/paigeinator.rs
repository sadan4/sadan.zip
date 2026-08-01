use std::{borrow::Cow, debug_assert_matches, time::Duration};

use anyhow::Result;
use bitflags::bitflags;
use futures_core::Stream;
use itertools::Itertools as _;
use serenity::{
	all::{
		ButtonStyle,
		CacheHttp,
		ComponentInteraction,
		ComponentInteractionCollector,
		Context,
		CreateActionRow,
		CreateAllowedMentions,
		CreateButton,
		CreateComponent,
		CreateContainer,
		CreateContainerComponent,
		CreateInteractionResponse,
		CreateInteractionResponseFollowup,
		CreateInteractionResponseMessage,
		CreateMessage,
		EditInteractionResponse,
		EditMessage,
		Message,
		MessageFlags,
		ReactionType,
		UserId,
	},
	futures::StreamExt as _,
	small_fixed_array::FixedString,
};
use thiserror::Error;
use tokio::{select, task::JoinSet, time::sleep};
use tracing::{debug, error, info, warn};

use crate::fw::CommandCtx;

bitflags! {
	#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
	pub struct PaigeinatorFlags: u8 {
		const NONE = 0;
		/// If set, the paigeinator will fork to an ephemeral message if
		/// the user that interacted with the paigeinator is not [`Paigeinator::creator`]
		const FORK_ON_OTHER_INTERACTION = 1 << 0;
		/// If set, the paigeinator will loop back to the first page after the last page
		const LOOP = 1 << 1;
		/// If set, no page numbers will be shown
		const HIDE_PAGE_NUMBERS = 1 << 2;
		/// if set, the original interaction was deferred so the paigeinator must followup instead of respond
		const WAS_DEFERRED = 1 << 3;
		/// If set, and the source was a message, the paigeinator will ping on reply
		const PING_ON_REPLY = 1 << 4;
		/// If set, the paigeinator will be ephemeral. This will only work if the source was an interaction
		const EPHEMERAL = 1 << 5;
	}
}

#[derive(Error, Debug)]
pub enum PaigeinatorError {
	#[error("Paigeinator has no pages")]
	NoPages,
	#[error(
		"Starting page {starting_page} is out of bounds for {num_pages} pages"
	)]
	StartingPageOutOfBounds {
		starting_page: usize,
		num_pages: usize,
	},
	#[error("Serenity Error: {0}")]
	Serenity(
		#[from]
		#[source]
		serenity::Error,
	),
}

type PE = PaigeinatorError;
type Flags = PaigeinatorFlags;

const ID_PREV: &str = "\x00PAIGEINATOR_PREV";
const ID_COUNTER: &str = "\x00PAIGEINATOR_COUNTER";
const ID_NEXT: &str = "\x00PAIGEINATOR_NEXT";

impl PaigeinatorFlags {
	/// [`Self::WAS_DEFERRED`] | [`Self::FORK_ON_OTHER_INTERACTION`] | [`Self::LOOP`]
	pub const DEFAULT: Self = Self::NONE
		.union(Self::WAS_DEFERRED)
		.union(Self::FORK_ON_OTHER_INTERACTION)
		.union(Self::LOOP);
}

impl Default for PaigeinatorFlags {
	/// See: [`Self::DEFAULT`]
	fn default() -> Self {
		Self::DEFAULT
	}
}

#[derive(Debug, Default)]
pub struct Paigeinator<'a> {
	pages: Vec<Cow<'a, [CreateContainerComponent<'a>]>>,
	/// the current page the paigeinator is on, 0-indexed
	current_page: usize,
	/// The user that started this interaction
	///
	/// If this is set, only this user can interact with the paigeinator
	creator: Option<UserId>,
	/// flags. See: [`PaigeinatorFlags`]
	flags: PaigeinatorFlags,
	timeout: Option<Duration>,
	/// forks of this paigeinator
	forks: JoinSet<Result<(), PE>>,
}

impl<'a> Paigeinator<'a> {
	pub fn new() -> Self {
		Self::default()
	}
	/// set the passed flags
	pub fn with_flags(mut self, flags: Flags) -> Self {
		self.flags |= flags;
		self
	}
	/// unset the passed flags
	pub fn without_flags(mut self, flags: Flags) -> Self {
		self.flags &= !flags;
		self
	}
	pub const fn with_creator(mut self, creator: UserId) -> Self {
		self.creator = Some(creator);
		self
	}

	pub fn starting_page(mut self, page: usize) -> Self {
		if cfg!(debug_assertions) && page >= self.pages.len() {
			warn!(
				"Paigeinator::starting_page called with an out-of-bounds page index. This will error if no pages are added before the paigeinator is started."
			);
		}
		self.current_page = page;
		self
	}
	/// overwrites existing pages
	pub fn with_pages(
		mut self,
		pages: Vec<Cow<'a, [CreateContainerComponent<'a>]>>,
	) -> Self {
		if cfg!(debug_assertions) && !self.pages.is_empty() {
			warn!(
				"Paigeinator::with_pages called on a paigeinator that already has pages. This will overwrite the existing pages."
			);
		}
		self.pages = pages;
		self
	}
	pub fn add_page(
		mut self,
		page: impl Into<Cow<'a, [CreateContainerComponent<'a>]>>,
	) -> Self {
		self.pages.push(page.into());
		self
	}
	pub const fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = Some(timeout);
		self
	}
	const DEFAULT_TIMEOUT: Duration = Duration::from_mins(5);

	pub async fn run(
		mut self,
		c: &Context,
		cctx: &CommandCtx<'_>,
	) -> Result<(), PE> {
		self.validate()?;
		let msg_id = match cctx {
			CommandCtx::Prefix { msg } => {
				let cm = self.build_message_response(msg);
				msg.channel_id
					.send_message(c.http(), cm)
					.await?
					.id
			}
			CommandCtx::Application { interaction } => {
				if self.flags.contains(Flags::WAS_DEFERRED) {
					interaction
						.create_followup(
							c.http(),
							CreateInteractionResponseFollowup::new()
								.flags(MessageFlags::IS_COMPONENTS_V2)
								.ephemeral(
									self.flags.contains(Flags::EPHEMERAL),
								)
								.components(Cow::Owned(
									self.build_components(true),
								)),
						)
						.await?
						.id
				} else {
					interaction
						.create_response(
							c.http(),
							CreateInteractionResponse::Message(
								self.create_interaction_content(),
							),
						)
						.await?;
					interaction
						.get_response(c.http())
						.await?
						.id
				}
			}
		};
		let evts = ComponentInteractionCollector::new(c)
			.message_id(msg_id)
			.stream();
		self.run_loop(c, evts).await?;
		let final_content = self.build_components(false);
		if let Err(e) = match cctx {
			CommandCtx::Prefix { msg } => {
				msg.channel_id
					.edit_message(
						&c.http,
						msg_id,
						EditMessage::new().components(final_content),
					)
					.await
			}
			CommandCtx::Application { interaction } => {
				interaction
					.edit_response(
						&c.http,
						EditInteractionResponse::new()
							.components(final_content),
					)
					.await
			}
		} {
			warn!("Failed to finalize paigeinator content: {e:?}");
		}
		while let Some(res) = self.forks.join_next().await {
			match res {
				Ok(Ok(())) => todo!(),
				Ok(Err(e)) => {
					error!("Forked paigeinator failed: {e:?}");
				}
				Err(e) => {
					error!("Forked paigeinator task failed to join: {e:?}");
				}
			}
		}
		Ok(())
	}
}

#[expect(clippy::multiple_inherent_impl)]
impl<'a> Paigeinator<'a> {
	async fn run_loop(
		&mut self,
		c: &Context,
		es: impl Stream<Item = ComponentInteraction>,
	) -> Result<(), PE> {
		let dur = self
			.timeout
			.unwrap_or(Self::DEFAULT_TIMEOUT);
		tokio::pin!(es);
		while let Some(i) = select! {
			e = es.next() => e,
			() = sleep(dur) => None,
		} {
			if let Some(c_id) = self.creator
				&& c_id != i.user.id
			{
				if self
					.flags
					.contains(Flags::FORK_ON_OTHER_INTERACTION)
				{
					self.fork(c, i);
				} else if let Err(e) = self
					.err_ephemeral(c, &i, "This Paigeinator is not for you!")
					.await
				{
					error!("Failed to deny paigeinator interaction: {e:?}");
				}
				continue;
			}
			let Some(delta) = self.get_delta(c, &i).await else {
				continue;
			};
			let new_paige = self.calc_idx(delta);
			self.current_page = if new_paige == self.current_page {
				continue;
			} else {
				new_paige
			};
			self.update_interaction(c, &i).await?;
		}
		Ok(())
	}
	async fn get_delta(
		&self,
		c: &Context,
		i: &ComponentInteraction,
	) -> Option<i8> {
		let btn_id = i.data.custom_id.as_str();
		let delta: i8 = if btn_id == ID_PREV {
			-1
		} else if btn_id == ID_NEXT {
			1
		} else {
			if let Err(e) = self
				.err_ephemeral(
					c,
					i,
					&format!("Failed to revolve button id {btn_id:?}"),
				)
				.await
			{
				error!("Failed to send paigeinator error message: {e:?}");
			}
			return None;
		};
		Some(delta)
	}
	async fn update_interaction(
		&self,
		c: &Context,
		i: &ComponentInteraction,
	) -> Result<(), PE> {
		let r = CreateInteractionResponse::UpdateMessage(
			self.create_interaction_content(),
		);
		i.create_response(&c.http, r).await?;
		Ok(())
	}
	/// Calculates the next page index given a delta (-1 | 1)
	///
	/// if [`Self::flags`] contains [`Flags::LOOP`] the index will wrap instead of nooping
	fn calc_idx(&self, delta: i8) -> usize {
		let len = self.pages.len();
		let cur = self.current_page;
		debug_assert_matches!(delta, -1 | 1, "delta must be -1 or 1");
		if delta == -1 {
			if cur == 0 {
				if self.flags.contains(Flags::LOOP) {
					len - 1
				} else {
					0
				}
			} else {
				cur - 1
			}
		} else {
			if cur == len - 1 {
				if self.flags.contains(Flags::LOOP) {
					0
				} else {
					len - 1
				}
			} else {
				cur + 1
			}
		}
	}
	fn detach_paiges(
		&self,
	) -> Vec<Cow<'static, [CreateContainerComponent<'static>]>> {
		self.pages
			.clone()
			.into_iter()
			.map(|p| {
				p.into_owned()
					.into_iter()
					.map(CreateContainerComponent::into_owned)
					.collect_vec()
					.into()
			})
			.collect_vec()
	}
	fn fork(&mut self, c: &Context, i: ComponentInteraction) {
		debug!("Forking paigeinator for user {}", i.user.name);
		let new_ctx = c.clone();
		let mut new_paigeinator = Paigeinator {
			pages: self.detach_paiges(),
			// this is ephemeral, so the user id doesn't matter
			creator: None,
			current_page: self.current_page,
			flags: self.flags | Flags::EPHEMERAL,
			timeout: self.timeout,
			forks: JoinSet::new(),
		};
		self.forks.spawn(async move {
			new_paigeinator.validate()?;
			new_paigeinator.current_page = new_paigeinator
				.get_delta(&new_ctx, &i)
				.await
				.map_or(new_paigeinator.current_page, |d| {
					new_paigeinator.calc_idx(d)
				});
			i.create_response(
				&new_ctx.http,
				CreateInteractionResponse::Message(
					new_paigeinator.create_interaction_content(),
				),
			)
			.await?;
			let msg_id = i.get_response(&new_ctx.http).await?.id;
			let evts = ComponentInteractionCollector::new(&new_ctx)
				.message_id(msg_id)
				.stream();
			new_paigeinator
				.run_loop(&new_ctx, evts)
				.await?;
			let final_content = new_paigeinator.build_components(false);
			if let Err(e) = i
				.edit_response(
					&new_ctx.http,
					EditInteractionResponse::new().components(final_content),
				)
				.await
			{
				warn!("Failed to finalize pagieinator content in fork: {e:?}");
			}
			info!("Forked paigeinator for user {} completed", i.user.name);
			Ok(())
		});
	}
	async fn err_ephemeral(
		&self,
		c: &Context,
		i: &ComponentInteraction,
		msg: &str,
	) -> Result<(), PE> {
		i.create_response(
			&c.http,
			CreateInteractionResponse::Message(
				CreateInteractionResponseMessage::new()
					.content(msg)
					.ephemeral(true),
			),
		)
		.await?;
		Ok(())
	}
	const fn validate(&self) -> Result<(), PE> {
		if self.pages.is_empty() {
			Err(PE::NoPages)
		} else if self.current_page >= self.pages.len() {
			Err(PE::StartingPageOutOfBounds {
				starting_page: self.current_page,
				num_pages: self.pages.len(),
			})
		} else {
			Ok(())
		}
	}

	fn build_buttons(&self) -> CreateComponent<'a> {
		let mut ret = Vec::with_capacity(3);
		let prev = CreateButton::new(ID_PREV)
			.style(ButtonStyle::Secondary)
			.emoji(ReactionType::Unicode(FixedString::from_static_trunc("⬅️")));
		ret.push(prev);
		if !self
			.flags
			.contains(Flags::HIDE_PAGE_NUMBERS)
		{
			let counter = CreateButton::new(ID_COUNTER)
				.label(format!(
					"{}/{}",
					self.current_page + 1,
					self.pages.len()
				))
				.style(ButtonStyle::Secondary)
				.disabled(true);
			ret.push(counter);
		}
		let next = CreateButton::new(ID_NEXT)
			.style(ButtonStyle::Secondary)
			.emoji(ReactionType::Unicode(FixedString::from_static_trunc("➡️")));
		ret.push(next);
		CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(ret)))
	}

	fn build_components(&self, with_buttons: bool) -> Vec<CreateComponent<'a>> {
		let mut ret = Vec::with_capacity(usize::from(with_buttons) + 1);
		let contents = match &self.pages[self.current_page] {
			Cow::Borrowed(b) => Vec::from(*b),
			Cow::Owned(o) => o.clone(),
		};
		ret.push(CreateComponent::Container(CreateContainer::new(
			Cow::Owned(contents),
		)));
		if with_buttons {
			ret.push(self.build_buttons());
		}
		ret
	}

	fn create_interaction_content(
		&self,
	) -> CreateInteractionResponseMessage<'a> {
		CreateInteractionResponseMessage::new()
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(Cow::Owned(self.build_components(true)))
			.ephemeral(self.flags.contains(Flags::EPHEMERAL))
	}

	fn build_message_response(&self, orig_msg: &Message) -> CreateMessage<'a> {
		let mut cm = CreateMessage::new()
			.flags(MessageFlags::IS_COMPONENTS_V2)
			.components(self.build_components(true));
		if self
			.flags
			.contains(Flags::PING_ON_REPLY)
		{
			cm = cm
				.allowed_mentions(
					CreateAllowedMentions::new().replied_user(true),
				)
				.reference_message(orig_msg);
		}
		cm
	}
}
