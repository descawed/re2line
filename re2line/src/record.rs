use std::io::{Read, Seek, SeekFrom};

use anyhow::{Result, bail};
use binrw::BinReaderExt;
use re2shared::record::*;

#[derive(Debug)]
pub struct Recording {
    version: u16,
    frames: Vec<FrameRecord>,
}

impl Recording {
    pub fn read(mut f: impl Read + Seek + BinReaderExt) -> Result<Self> {
        let size = f.seek(SeekFrom::End(0))?;
        f.seek(SeekFrom::Start(0))?;

        let header: RecordHeader = f.read_le()?;
        if header.version != RECORD_VERSION {
            bail!("Unsupported record version {}", header.version);
        }

        let mut frames: Vec<FrameRecord> = Vec::new();
        while f.seek(SeekFrom::Current(0))? < size {
            frames.push(f.read_le()?);
        }

        Ok(Self {
            version: header.version,
            frames,
        })
    }
}