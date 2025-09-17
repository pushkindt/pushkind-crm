use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::client_event::{ClientEvent, NewClientEvent};
use crate::domain::manager::Manager;
use crate::models::client_event::{
    ClientEvent as DbClientEvent, NewClientEvent as DbNewClientEvent,
};
use crate::models::manager::Manager as DbManager;
use crate::repository::{
    ClientEventListQuery, ClientEventReader, ClientEventWriter, DieselRepository,
};

impl ClientEventReader for DieselRepository {
    fn list_client_events(
        &self,
        query: ClientEventListQuery,
    ) -> RepositoryResult<(usize, Vec<(ClientEvent, Manager)>)> {
        use crate::schema::{client_events, managers};
        use std::collections::{HashMap, HashSet};

        let mut conn = self.conn()?;

        let query_builder = || {
            // Start with boxed query on clients
            let mut items = client_events::table
                .filter(client_events::client_id.eq(query.client_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(event_type) = &query.event_type {
                items = items.filter(client_events::event_type.eq(event_type.to_string()));
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

        // --- 4. Load the events ---
        let db_events = items
            .order(client_events::created_at.desc())
            .load::<DbClientEvent>(&mut conn)?;

        // --- 5. Load the managers using IN clause ---
        let manager_ids: Vec<i32> = db_events.iter().map(|e| e.manager_id).collect();
        let unique_manager_ids: Vec<i32> = {
            let set: HashSet<_> = manager_ids.into_iter().collect();
            set.into_iter().collect()
        };

        let db_managers = managers::table
            .filter(managers::id.eq_any(unique_manager_ids))
            .load::<DbManager>(&mut conn)?;

        // --- 6. Map managers by id ---
        let manager_map: HashMap<i32, DbManager> =
            db_managers.into_iter().map(|m| (m.id, m)).collect();

        // --- 7. Combine events with managers ---
        let combined: Vec<(ClientEvent, Manager)> = db_events
            .into_iter()
            .filter_map(|event| {
                manager_map
                    .get(&event.manager_id)
                    .map(|manager| (event.into(), manager.clone().into()))
            })
            .collect();

        Ok((total, combined))
    }
}

impl ClientEventWriter for DieselRepository {
    fn create_client_event(&self, client_event: &NewClientEvent) -> RepositoryResult<ClientEvent> {
        use crate::schema::client_events;

        let mut conn = self.conn()?;

        let new_client_event: DbNewClientEvent = client_event.into();

        if let Some(existing_event) = client_events::table
            .filter(client_events::client_id.eq(new_client_event.client_id))
            .filter(client_events::manager_id.eq(new_client_event.manager_id))
            .filter(client_events::event_type.eq(&new_client_event.event_type))
            .filter(client_events::event_data.eq(&new_client_event.event_data))
            .order(client_events::created_at.desc())
            .first::<DbClientEvent>(&mut conn)
            .optional()?
        {
            return Ok(existing_event.into());
        }

        let client_event = diesel::insert_into(client_events::table)
            .values(&new_client_event)
            .get_result::<DbClientEvent>(&mut conn)?;

        Ok(client_event.into())
    }
}
