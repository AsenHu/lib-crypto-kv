use std::{collections::HashSet, path::Path};

use sled::{IVec, Tree};

use crate::metadata::{Metadata, ToMetadata};

#[derive(Clone)]
pub struct Db {
    metadata: Tree,
    data: Tree,
}

pub struct Object {
    pub data: IVec,
    pub metadata: Metadata,
}

impl Db {
    pub fn open(
        path: impl AsRef<Path>,
        mode: sled::Mode,
        compression_factor: i32,
        cache_capacity: u64,
    ) -> Result<Self, sled::Error> {
        let db = sled::Config::new()
            .cache_capacity(cache_capacity)
            .mode(mode)
            .compression_factor(compression_factor)
            .path(path.as_ref())
            .open()?;
        Ok(Self {
            metadata: db.open_tree("metadata")?,
            data: db.open_tree("data")?,
        })
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Object>, sled::Error> {
        let (metadata, value) = match (self.metadata.get(key)?, self.data.get(key)?) {
            (Some(metadata), Some(value)) => (metadata.to_metadata(), value),
            _ => return Ok(None),
        };
        Ok(Some(Object {
            data: value,
            metadata,
        }))
    }

    pub fn insert(&self, key: &[u8], obj: Object) -> Result<Option<Object>, sled::Error> {
        let (metadata, value) = match (
            self.metadata.insert(key, obj.metadata.into_boxed_slice())?,
            self.data.insert(key, obj.data)?,
        ) {
            (Some(metadata), Some(value)) => (metadata.to_metadata(), value),
            _ => return Ok(None),
        };
        Ok(Some(Object {
            data: value,
            metadata: metadata,
        }))
    }

    pub fn remove(&self, key: &[u8]) -> Result<Option<Object>, sled::Error> {
        let (metadata, value) = match (self.metadata.remove(key)?, self.data.remove(key)?) {
            (Some(metadata), Some(value)) => (metadata.to_metadata(), value),
            _ => return Ok(None),
        };
        Ok(Some(Object {
            data: value,
            metadata,
        }))
    }

    pub fn reclaim_outdated(&self) -> Result<HashSet<Box<str>>, sled::Error> {
        let mut keys = HashSet::new();
        for result in &self.metadata {
            let (key, value) = result?;
            let metadata = value.to_metadata();
            if metadata.is_outdated() {
                self.metadata.remove(&key)?;
                self.data.remove(&key)?;
                keys.insert(String::from_utf8_lossy(&key).into_owned().into_boxed_str());
            }
        }
        Ok(keys)
    }
}

impl Object {
    pub fn new(data: IVec, metadata: Metadata) -> Self {
        Self { data, metadata }
    }
}
