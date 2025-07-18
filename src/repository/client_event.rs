use diesel::prelude::*;

use crate::db::DbPool;
use crate::domain::client_event::{ClientEvent, NewClientEvent};
use crate::models::client_event::{
    ClientEvent as DbClientEvent, NewClientEvent as DbNewClientEvent,
};
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, errors::RepositoryResult,
};

/// Diesel implementation of [`ClientEventRepository`].
pub struct DieselClientEventRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselClientEventRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl<'a> ClientEventReader for DieselClientEventRepository<'a> {
    fn list(&self, query: ClientEventListQuery) -> RepositoryResult<(usize, Vec<ClientEvent>)> {
        use crate::schema::client_events;

        let mut conn = self.pool.get().unwrap();

        let query_builder = || {
            // Start with boxed query on clients
            let mut items = client_events::table
                .filter(client_events::client_id.eq(query.client_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(event_type) = &query.event_type {
                items = items.filter(client_events::event_data.eq(event_type.to_string()));
            }
            items
        };

        // Get the total count before applying pagination
        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();
        // Apply pagination if requested
        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        // Final load
        let items = items
            .order(client_events::created_at.desc())
            .load::<DbClientEvent>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<ClientEvent>>();

        Ok((total, items))
    }
}

impl<'a> ClientEventWriter for DieselClientEventRepository<'a> {
    fn create(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent> {
        use crate::schema::client_events;

        let mut conn = self.pool.get().unwrap();

        let new_client_event: DbNewClientEvent = client_event.into();

        let client_event = diesel::insert_into(client_events::table)
            .values(&new_client_event)
            .get_result::<DbClientEvent>(&mut conn)?;

        Ok(client_event.into())
    }
}
