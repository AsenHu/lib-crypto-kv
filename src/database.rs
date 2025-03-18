use std::{collections::HashSet, path::PathBuf};

use sled::{IVec, Tree};

use crate::metadata::{Metadata, ToMetadata};

#[derive(Debug, Clone)]
pub struct Db {
    metadata: Tree,
    data: Tree,
}

pub fn open(path: impl Into<PathBuf>) -> Result<Db, sled::Error> {
    Db::open(path)
}

pub struct Object {
    pub data: IVec,
    pub metadata: Metadata,
}

impl Db {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, sled::Error> {
        let db = sled::Config::new()
            .compression_factor(22)
            .path(path.into())
            .open()?;
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

    pub fn reclaim_outdated(&self) -> Result<HashSet<IVec>, sled::Error> {
        let mut keys = HashSet::new();
        for result in &self.metadata {
            let (key, value) = result?;
            let metadata = value.to_metadata();
            if metadata.is_outdated() {
                self.metadata.remove(&key)?;
                self.data.remove(&key)?;
                keys.insert(key);
            }
        }
        Ok(keys)
    }
}
