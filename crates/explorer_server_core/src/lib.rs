use anyhow::{Result, bail};
use explorer_types::FullBundle;
use std::{
    env,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
};

pub const DATA_FILE_NAME: &str = "data.mpk.zst";
pub const METADATA_FILE_NAME: &str = "meta.mpk.zst";

pub fn get_root_build_path() -> Result<PathBuf> {
    let build_path = env::current_dir()?.join("builds");
    if !build_path.exists() {
        fs::create_dir_all(&build_path)?;
    }
    Ok(build_path)
}

pub fn get_build_path(build_hash: &str) -> Result<PathBuf> {
    Ok(get_root_build_path()?.join(build_hash))
}

pub fn is_build_downloaded(build_hash: &str) -> Result<bool> {
    Ok(get_build_path(build_hash)?.is_dir())
}

pub fn get_version_file_path() -> Result<PathBuf> {
    Ok(get_root_build_path()?.join(".ver"))
}

pub fn build_has_data(build_hash: &Path) -> bool {
    build_hash.join(DATA_FILE_NAME).is_file()
}

pub fn write_full_bundle(bundle: &FullBundle) -> Result<()> {
    let build_path = get_build_path(&bundle.metadata.build_hash)?;

    if !build_path.exists() {
        fs::create_dir_all(&build_path)?;
    }

    let meta_bin = rmp_serde::to_vec(&bundle.metadata)?;
    let meta_zst = zstd::encode_all(meta_bin.as_slice(), 0)?;
    drop(meta_bin);
    fs::write(build_path.join(METADATA_FILE_NAME), meta_zst)?;
    let data_mpk = rmp_serde::to_vec(&bundle)?;
    let data_zst = compress_full_bundle_data(&data_mpk)?;
    drop(data_mpk);

    fs::write(build_path.join(DATA_FILE_NAME), data_zst)?;

    Ok(())
}

pub fn compress_full_bundle_data(data: &[u8]) -> Result<Vec<u8>> {
    Ok(zstd::encode_all(data, 10)?)
}

trait Encodable {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize>;
    fn from_bts(w: &mut impl io::Read) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Channel {
    Stable = 0,
    Canary = 1,
}

impl Encodable for Channel {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize> {
        let buf = [*self as u8];
        w.write_all(&buf)?;
        Ok(buf.len())
    }
    fn from_bts(w: &mut impl io::Read) -> Result<Self> {
        const STABLE: u8 = Channel::Stable as u8;
        const CANARY: u8 = Channel::Canary as u8;

        let mut buf = [u8::MAX];
        w.read_exact(&mut buf)?;
        match buf[0] {
            STABLE => Ok(Self::Stable),
            CANARY => Ok(Self::Canary),
            v => bail!("invalid channel value: {v}"),
        }
    }
}

impl Encodable for String {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize> {
        let bts = self.as_bytes();
        let len = bts.len();
        let cnt = len + len.to_bts(w)?;
        w.write_all(bts)?;
        Ok(cnt)
    }
    fn from_bts(w: &mut impl io::Read) -> Result<Self> {
        let len = usize::from_bts(w)?;
        let mut buf = vec![0u8; len];
        w.read_exact(&mut buf)?;
        let res = Self::from_utf8(buf)?;
        Ok(res)
    }
}

impl Encodable for usize {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize> {
        let buf = self.to_ne_bytes();
        w.write_all(&buf)?;
        Ok(buf.len())
    }
    fn from_bts(w: &mut impl io::Read) -> Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; _];
        w.read_exact(&mut buf)?;
        Ok(Self::from_ne_bytes(buf))
    }
}

/// self.0 must always be 40 hex chars
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Sha1Hash([u8; 40]);

impl Debug for Sha1Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Sha1Hash").field(&self.as_ref()).finish()
    }
}

impl TryFrom<&str> for Sha1Hash {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        if !value.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("not a valid sha1 hash: invalid characters");
        }
        let value = value.as_bytes();
        if value.len() != 40 {
            bail!("not a valid sha1 hash: invalid length");
        }
        let mut buf = [0u8; 40];
        buf.copy_from_slice(value);
        Ok(Self(buf))
    }
}

impl Encodable for Sha1Hash {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize> {
        w.write_all(&self.0)?;
        Ok(self.0.len())
    }
    fn from_bts(w: &mut impl io::Read) -> Result<Self> {
        let mut buf = [0u8; 40];
        w.read_exact(&mut buf)?;
        if buf.iter().all(u8::is_ascii_hexdigit) {
            Ok(Self(buf))
        } else {
            bail!("not a valid sha1 hash: invalid characters")
        }
    }
}

impl From<Sha1Hash> for String {
    fn from(value: Sha1Hash) -> Self {
        unsafe { Self::from_utf8_unchecked(value.0.to_vec()) }
    }
}

impl AsRef<str> for Sha1Hash {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EncodableBuild {
    pub channel: Channel,
    pub build_hash: Sha1Hash,
    pub html: String,
}

impl Encodable for EncodableBuild {
    fn to_bts(&self, w: &mut impl io::Write) -> Result<usize> {
        let mut cnt = 0;
        cnt += self.channel.to_bts(w)?;
        cnt += self.build_hash.to_bts(w)?;
        cnt += self.html.to_bts(w)?;
        Ok(cnt)
    }

    fn from_bts(w: &mut impl io::Read) -> Result<Self>
    where
        Self: Sized,
    {
        let channel = Encodable::from_bts(w)?;
        let build_hash = Encodable::from_bts(w)?;
        let html = Encodable::from_bts(w)?;
        Ok(Self {
            channel,
            build_hash,
            html,
        })
    }
}

impl EncodableBuild {
    pub fn encode(&self, to: &mut impl io::Write) -> Result<()> {
        self.to_bts(to)?;
        Ok(())
    }
    pub fn decode(from: &mut impl io::Read) -> Result<Self> {
        Self::from_bts(from)
    }
}
