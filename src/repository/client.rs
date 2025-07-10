use diesel::prelude::*;

use crate::{
    db::DbPool,
    domain::client::Client,
    repository::{ClientRepository, errors::RepositoryResult},
};

/// Diesel implementation of [`ClientRepository`].
pub struct DieselClientRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselClientRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}
