use std::{ffi::OsStr, path::PathBuf, time::Instant};

use anyhow::{Context as _, Result};
use tokio::{fs, try_join};
use tracing::debug;
use typesize::derive::TypeSize;

#[derive(TypeSize)]
pub struct GifTemplates {
	pub hammer: GifTemplate,
}

async fn init_gif_templates(
	cfg: &bot_config::GifTemplates,
) -> Result<GifTemplates> {
	let start = Instant::now();
	let bot_config::GifTemplates { hammer } = cfg;
	let hammer = GifTemplate::try_from_config(hammer);
	let (hammer,) = try_join!(hammer)?;
	debug!("loaded gif templates from disk in {:.2?}", start.elapsed());
	Ok(GifTemplates { hammer })
}

#[derive(TypeSize)]
pub struct GifTemplate {
	pub config: bot_config::GifTemplateData,
	pub frames: Vec<Vec<u8>>,
}

async fn load_frames(
	dir: impl AsRef<OsStr>,
	tmpl: &bot_config::GifTemplateData,
) -> Result<Vec<Vec<u8>>> {
	let mut dir = PathBuf::from(dir.as_ref());
	dir.push("guh");
	let mut frames = Vec::with_capacity(tmpl.num_frames as usize);
	for i in 0..tmpl.num_frames {
		dir.pop();
		dir.push(format!(
			"{prefix}{i}{ext}",
			prefix = tmpl.frame_prefix,
			ext = tmpl.file_type.ext()
		));
		let frame = fs::read(&dir).await.with_context(|| {
			format!("Failed to read frame from {}", dir.display())
		})?;
		frames.push(frame);
	}
	Ok(frames)
}

impl GifTemplate {
	pub async fn try_from_config(
		cfg_dirs: &bot_config::GifTemplate,
	) -> Result<Self> {
		let data = fs::read(&cfg_dirs.data_path)
			.await
			.with_context(|| {
				format!(
					"Failed to read GIF template data path from {}",
					cfg_dirs.data_path.display()
				)
			})?;
		let config = serde_json::from_slice(&data)
			.context("Failed to deserialize GIF template data")?;
		let frames = load_frames(&cfg_dirs.frames_path, &config)
			.await
			.context("Failed to load frames")?;
		Ok(Self { config, frames })
	}
}

impl super::CommandFramework {
	/// Get the gif templates, load if needed
	pub async fn get_gif_templates(&self) -> Result<&GifTemplates> {
		self.gif_templates
			.get_or_try_init(|| {
				init_gif_templates(&self.config.assets.gif_templates)
			})
			.await
			.context("Failed to init gif templates")
	}
}
