use bincode::config::Configuration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use sled::IVec;

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    last_modified: DateTime<Utc>,
    delete_after_days: u16,
    dgst: Box<[u8]>,
    salt: Box<[u8]>,
    pass: Box<[u8]>,
}

fn pass(k: &str, salt: &[u8], p: &str) -> Box<[u8]> {
    Sha3_512::digest([k.as_bytes(), salt, p.as_bytes()].concat())
        .as_slice()
        .into()
}

impl Metadata {
    pub fn new(k: &str, v: &[u8], p: &str) -> Self {
        let dgst: Box<[u8]> = Sha3_512::digest(v).as_slice().into();
        let salt: Box<[u8]> = rand::random::<[u8; 64]>().into();
        let pass = pass(k, &salt, p);
        Self {
            last_modified: Utc::now(),
            delete_after_days: 7,
            dgst,
            salt,
            pass,
        }
    }

    pub fn modifiy(&mut self, v: &[u8]) {
        self.last_modified = Utc::now();
        self.dgst = Sha3_512::digest(v).as_slice().into();
    }

    pub fn set_auto_del(&mut self, days: u16) {
        self.delete_after_days = days;
    }

    pub fn is_allowed(&self, k: &str, p: &str) -> bool {
        let pass = pass(k, &self.salt, p);
        self.pass == pass
    }

    pub fn last_modified(&self) -> &DateTime<Utc> {
        &self.last_modified
    }

    pub fn is_outdated(&self) -> bool {
        let days = Utc::now()
            .signed_duration_since(self.last_modified)
            .num_days();
        days > self.delete_after_days.into()
    }

    pub fn dgst(&self) -> &[u8] {
        &self.dgst
    }

    pub fn into_boxed_slice(self) -> Box<[u8]> {
        bincode::serde::encode_to_vec::<Metadata, Configuration>(self, Configuration::default())
            .unwrap()
            .into()
    }
}

pub trait ToMetadata {
    fn to_metadata(self) -> Metadata;
}

impl ToMetadata for IVec {
    fn to_metadata(self) -> Metadata {
        let (value, _) = bincode::serde::decode_from_slice::<Metadata, Configuration>(
            &self,
            Configuration::default(),
        )
        .unwrap();
        value
    }
}
