use std::collections::BTreeSet;
use std::path::Path;

use rocksdb::{ColumnFamily, Options, DB};
use std::sync::Arc;

use crate::db::cf_map::BONSAI_COLUMNS;

#[derive(Debug, thiserror::Error)]
pub enum DbOpenError {
    #[error("rocksdb error: {0}")]
    Rocks(#[from] rocksdb::Error),
    #[error("missing required column families: {0:?}")]
    MissingCfs(Vec<String>),
}

#[derive(Debug)]
pub struct RocksDb {
    db: Arc<DB>,
    cf_names: Vec<String>,
}

impl Clone for RocksDb {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            cf_names: self.cf_names.clone(),
        }
    }
}

impl RocksDb {
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, DbOpenError> {
        let path = path.as_ref();
        let mut opts = Options::default();
        opts.set_max_open_files(256);

        let cf_names = DB::list_cf(&opts, path)?;
        let cf_set: BTreeSet<_> = cf_names.iter().cloned().collect();

        let missing: Vec<String> = BONSAI_COLUMNS
            .iter()
            .filter(|name| !cf_set.contains(**name))
            .map(|name| name.to_string())
            .collect();

        if !missing.is_empty() {
            return Err(DbOpenError::MissingCfs(missing));
        }

        let db = Arc::new(DB::open_cf_for_read_only(&opts, path, &cf_names, false)?);

        Ok(Self { db, cf_names })
    }

    pub fn cf_names(&self) -> &[String] {
        &self.cf_names
    }

    pub fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.db.cf_handle(name)
    }

    pub fn get_cf(&self, name: &str, key: &[u8]) -> Result<Option<Vec<u8>>, rocksdb::Error> {
        let Some(cf) = self.db.cf_handle(name) else {
            return Ok(None);
        };
        self.db.get_cf(cf, key)
    }

    pub fn iter_cf_from(
        &self,
        name: &str,
        prefix: &[u8],
    ) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, rocksdb::Error> {
        let Some(cf) = self.db.cf_handle(name) else {
            return Ok(Box::new(Vec::<(Vec<u8>, Vec<u8>)>::new().into_iter()));
        };
        let iter = self
            .db
            .iterator_cf(cf, rocksdb::IteratorMode::From(prefix, rocksdb::Direction::Forward))
            .filter_map(|res| res.ok())
            .map(|(k, v)| (k.to_vec(), v.to_vec()));
        Ok(Box::new(iter))
    }
}
