use std::net::SocketAddr;

use anyhow::Context;
use explorer_server_core::{METADATA_FILE_NAME, get_root_build_path};
use explorer_types::{
	BuildHashQuery,
	Bundle7zArchive,
	BundleMetadata,
	FullBundle,
	GitHash,
	TimestampQueryResults,
	build_archive_server::{BuildArchive, BuildArchiveServer},
	google,
};
use prost::Message as _;
use tokio::{fs, sync::mpsc};
use tokio_stream::{Stream, StreamExt as _, wrappers::{ReadDirStream, ReceiverStream}};
use tonic::{
	Code,
	Response,
	Status,
	async_trait,
	codec::CompressionEncoding,
	transport::Server,
};
use tracing::{error, warn};

use crate::State;

type O<T> = tonic::Result<tonic::Response<T>>;

pub struct BuildServiceImpl {
	state: State,
}

impl BuildServiceImpl {
	pub fn start(bind_addr: SocketAddr, state: State) {
		let handler = Self { state };
		let server = Server::builder().add_service(
			BuildArchiveServer::new(handler)
				.accept_compressed(CompressionEncoding::Zstd)
				.send_compressed(CompressionEncoding::Gzip)
				.send_compressed(CompressionEncoding::Zstd)
				.send_compressed(CompressionEncoding::Gzip),
		);
		tokio::spawn(async move {
			if let Err(e) = server
				.serve(bind_addr)
				.await
				.context("Failed to run build service")
			{
				error!("Build service failed: {e:?}");
			} else {
				warn!("Build service exited cleanly");
			}
		});
	}
}

#[async_trait]
impl BuildArchive for BuildServiceImpl {
	async fn get_build_metadata(
		&self,
		request: tonic::Request<BuildHashQuery>,
	) -> O<BundleMetadata> {
		todo!()
	}
	async fn get_build_archive(
		&self,
		request: tonic::Request<BuildHashQuery>,
	) -> O<FullBundle> {
		todo!()
	}
	async fn get_bundle7z_archive(
		&self,
		request: tonic::Request<BuildHashQuery>,
	) -> O<Bundle7zArchive> {
		todo!()
	}
	type ListBuildsStream = ReceiverStream<tonic::Result<BundleMetadata>>;

	async fn list_builds(
		&self,
		_: tonic::Request<google::protobuf::Empty>,
	) -> O<Self::ListBuildsStream> {
		let (tx, rx) = mpsc::channel(128);
		tokio::spawn(async move {
			let res = try {
				let dirs = fs::read_dir(get_root_build_path()?)
					.await
					.context("Failed to read root build path dir")?;
				let mut st = ReadDirStream::new(dirs);
				while let Some(p) = st.next().await {
					let p = p.context("Failed to read directory entry")?;
					let tx = tx.clone();
					tokio::task::spawn(async move {
						let res = try {
							if !p.metadata().await.context("Failed to read metadata for directory entry")?.is_dir() {
								return;
							}
							let mut meta_path = p.path();
							meta_path.push(METADATA_FILE_NAME);
							if !fs::try_exists(&meta_path).await.context("Failed to stat meta path")? {
								return;
							}
							let bts = fs::read(&meta_path).await.context("Failed to read metadata file")?;
							let meta = BundleMetadata::decode(& *bts).context("Failed to decode metadata file")?;
							tx.send(Ok(meta)).await.context("Failed to send metadata over channel")?;
						};
						if let Err(e) = res {
							_ = tx.send(Err(Status::new(Code::Internal, format!("{e}")))).await;
						}
					});
				}
			};
			if let Err(e) = res {
				_ = tx
					.send(Err(Status::new(Code::Internal, format!("{e}"))))
					.await;
			}
		});
		Ok(Response::new(ReceiverStream::new(rx)))
	}
	async fn get_builds_before_timestamp(
		&self,
		request: tonic::Request<google::protobuf::Timestamp>,
	) -> O<TimestampQueryResults> {
		todo!()
	}
	async fn get_builds_before_hash(
		&self,
		request: tonic::Request<BuildHashQuery>,
	) -> O<TimestampQueryResults> {
		todo!()
	}
	async fn get_latest_build(
		&self,
		request: tonic::Request<google::protobuf::Empty>,
	) -> O<BundleMetadata> {
		todo!()
	}
	async fn fixup_timestamps(
		&self,
		request: tonic::Request<google::protobuf::Empty>,
	) -> O<google::protobuf::Empty> {
		todo!()
	}
	async fn version(
		&self,
		request: tonic::Request<google::protobuf::Empty>,
	) -> O<GitHash> {
		todo!()
	}
}
