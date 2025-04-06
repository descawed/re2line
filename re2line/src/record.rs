use std::io::{Read, Seek, SeekFrom};

use anyhow::{Result, bail};
use binrw::BinReaderExt;
use re2shared::record::*;

#[derive(Debug)]
pub struct Recording {
    version: u16,
    frames: Vec<FrameRecord>,
    index: usize,
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
            index: 0,
        })
    }

    pub fn frames(&self) -> &[FrameRecord] {
        &self.frames
    }

    pub fn current(&self) -> Option<&FrameRecord> {
        self.frames.get(self.index)
    }

    pub fn next(&mut self) -> Option<&FrameRecord> {
        self.index += 1;
        self.current()
    }

    pub fn set_index(&mut self, index: usize) -> Option<&FrameRecord> {
        self.index = index;
        self.current()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}