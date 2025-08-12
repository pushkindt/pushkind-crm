use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};

use crate::{
    domain::client::{Client, NewClient, UpdateClient},
    domain::manager::Manager,
    models::client::{
        Client as DbClient, NewClient as DbNewClient, UpdateClient as DbUpdateClient,
    },
    models::manager::Manager as DbManager,
    repository::{
        ClientListQuery, ClientReader, ClientWriter, DieselRepository, errors::RepositoryResult,
    },
};

impl ClientReader for DieselRepository {
    fn get_client_by_id(&self, id: i32) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .find(id)
            .first::<DbClient>(&mut conn)
            .optional()?;

        Ok(client.map(Into::into))
    }

    fn list_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)> {
        use crate::schema::{client_manager, clients, managers};

        let mut conn = self.conn()?;

        let query_builder = || {
            // Start with boxed query on clients
            let mut items = clients::table
                .filter(clients::hub_id.eq(query.hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(manager_email) = &query.manager_email {
                items = items.filter(
                    clients::id.eq_any(
                        client_manager::table
                            .filter(
                                client_manager::manager_id.nullable().eq(managers::table
                                    .filter(managers::email.eq(manager_email))
                                    .filter(managers::hub_id.eq(query.hub_id))
                                    .select(managers::id)
                                    .single_value()),
                            )
                            .select(client_manager::client_id),
                    ),
                );
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
            .order(clients::id.asc())
            .load::<DbClient>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Client>>();

        Ok((total, items))
    }

    fn search_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)> {
        use crate::models::client::ClientCount;

        let mut conn = self.conn()?;

        let match_query = match &query.search {
            None => return Ok((0, vec![])),
            Some(query) if query.is_empty() => {
                return Ok((0, vec![]));
            }
            Some(query) => {
                format!("{query}*")
            }
        };

        // Build base SQL
        let mut sql = String::from(
            r#"
            SELECT clients.*
            FROM clients
            JOIN client_fts ON clients.id = client_fts.rowid
            WHERE client_fts MATCH ?
            AND clients.hub_id = ?
            "#,
        );

        if query.manager_email.is_some() {
            let manager_filter = r#"
                AND clients.id IN (
                    SELECT client_manager.client_id
                    FROM client_manager
                    JOIN managers ON client_manager.manager_id = managers.id
                    WHERE managers.email = ?
                    AND managers.hub_id = ?
                )
            "#;
            sql.push_str(manager_filter);
        }

        let total_sql = format!("SELECT COUNT(*) as count FROM ({sql})");

        // Now add pagination to SQL (but not count)
        if query.pagination.is_some() {
            sql.push_str(" LIMIT ? OFFSET ? ");
        }

        // Build final data query
        let mut data_query = diesel::sql_query(&sql)
            .into_boxed()
            .bind::<Text, _>(&match_query)
            .bind::<Integer, _>(query.hub_id);

        let mut total_query = diesel::sql_query(&total_sql)
            .into_boxed()
            .bind::<Text, _>(&match_query)
            .bind::<Integer, _>(query.hub_id);

        if let Some(manager_email) = &query.manager_email {
            data_query = data_query
                .bind::<Text, _>(manager_email)
                .bind::<Integer, _>(query.hub_id);
            total_query = total_query
                .bind::<Text, _>(manager_email)
                .bind::<Integer, _>(query.hub_id);
        }

        if let Some(pagination) = &query.pagination {
            let limit = pagination.per_page as i64;
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            data_query = data_query
                .bind::<BigInt, _>(limit)
                .bind::<BigInt, _>(offset);
        }

        let items = data_query
            .load::<DbClient>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect();

        let total = total_query.get_result::<ClientCount>(&mut conn)?.count as usize;
        Ok((total, items))
    }

    fn list_managers(&self, id: i32) -> RepositoryResult<Vec<Manager>> {
        use crate::schema::{client_manager, managers};
        let mut conn = self.conn()?;
        let managers = client_manager::table
            .filter(client_manager::client_id.eq(id))
            .inner_join(managers::table)
            .select(managers::all_columns)
            .load::<DbManager>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(managers)
    }

    fn check_client_assigned_to_manager(
        &self,
        client_id: i32,
        manager_email: &str,
    ) -> RepositoryResult<bool> {
        use crate::schema::{client_manager, clients, managers};
        let mut conn = self.conn()?;

        let assigned = client_manager::table
            .filter(client_manager::client_id.eq(client_id))
            .inner_join(managers::table)
            .filter(managers::email.eq(manager_email))
            .inner_join(clients::table)
            .filter(clients::id.eq(client_id))
            .filter(clients::hub_id.eq(managers::hub_id))
            .select(client_manager::client_id)
            .first::<i32>(&mut conn)
            .optional()?;
        Ok(assigned.is_some())
    }
}

impl ClientWriter for DieselRepository {
    fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let lower_emails: Vec<String> =
            new_clients.iter().map(|c| c.email.to_lowercase()).collect();

        let insertables: Vec<DbNewClient> = new_clients
            .iter()
            .zip(lower_emails.iter())
            .map(|(client, email)| DbNewClient {
                hub_id: client.hub_id,
                name: client.name.as_str(),
                email: email.as_str(),
                phone: client.phone.as_str(),
                address: client.address.as_str(),
            })
            .collect();
        let affected = diesel::insert_into(clients::table)
            .values(&insertables)
            .execute(&mut conn)?;
        Ok(affected)
    }

    fn update_client(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let email = updates.email.to_lowercase();
        let db_updates = DbUpdateClient {
            name: updates.name,
            email: email.as_str(),
            phone: updates.phone,
            address: updates.address,
        };

        let updated = diesel::update(clients::table.find(client_id))
            .set(&db_updates)
            .get_result::<DbClient>(&mut conn)?;
        Ok(updated.into())
    }

    fn delete_client(&self, client_id: i32) -> RepositoryResult<()> {
        use crate::schema::{client_manager, clients};

        let mut conn = self.conn()?;
        diesel::delete(client_manager::table.filter(client_manager::client_id.eq(client_id)))
            .execute(&mut conn)?;
        diesel::delete(clients::table.find(client_id)).execute(&mut conn)?;
        Ok(())
    }
}
