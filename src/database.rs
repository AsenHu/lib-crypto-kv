use std::path::PathBuf;

use log::info;

use crate::metadata::{Metadata, ToMetadata};

#[derive(Debug, Clone)]
pub struct Db {
    metadata: sled::Tree,
    data: sled::Tree,
}

pub fn open(path: impl Into<PathBuf>) -> Result<Db, sled::Error> {
    Db::open(path)
}

pub struct Object {
    pub data: sled::IVec,
    pub metadata: Metadata,
}

impl Db {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, sled::Error> {
        let db = sled::open(path.into())?;
        Ok(Self {
            metadata: db.open_tree("metadata")?,
            data: db.open_tree("data")?,
        })
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Object>, sled::Error> {
        let metadata = self.metadata.get(key)?;
        let metadata = match metadata {
            Some(metadata) => metadata.to_metadata(),
            None => return Ok(None),
        };
        let value = self.data.get(key)?;
        let value = match value {
            Some(value) => value,
            None => return Ok(None),
        };
        Ok(Some(Object {
            data: value,
            metadata,
        }))
    }

    pub fn insert(
        &self,
        key: &[u8],
        value: &[u8],
        metadata: Metadata,
    ) -> Result<Option<sled::IVec>, sled::Error> {
        self.metadata.insert(key, metadata.into_boxed_slice())?;
        self.data.insert(key, value)
    }

    pub fn remove(&self, key: &[u8]) -> Result<Option<sled::IVec>, sled::Error> {
        self.metadata.remove(key)?;
        self.data.remove(key)
    }
}

pub fn reclaim_outdated(db: &Db) -> Result<(), sled::Error> {
    let mut metadata = db.metadata.iter();
    while let Some(result) = metadata.next() {
        let (key, value) = result?;
        let metadata = value.to_metadata();
        if metadata.is_outdated() {
            info!("reclaiming outdated key: {:?}", key);
            db.metadata.remove(&key)?;
            db.data.remove(key)?;
        }
    }
    Ok(())
}
