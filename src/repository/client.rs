use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use diesel::upsert::excluded;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::models::client::ClientField;
use crate::{
    domain::client::{Client, NewClient, UpdateClient},
    domain::manager::Manager,
    models::client::{
        Client as DbClient, NewClient as DbNewClient, UpdateClient as DbUpdateClient,
    },
    models::manager::Manager as DbManager,
    repository::{ClientListQuery, ClientReader, ClientWriter, DieselRepository},
};

impl ClientReader for DieselRepository {
    fn get_client_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .find(id)
            .filter(clients::hub_id.eq(hub_id))
            .first::<DbClient>(&mut conn)
            .optional()?;
        let client = match client {
            Some(client) => client,
            None => return Ok(None),
        };

        let fields = ClientField::belonging_to(&client)
            .select(ClientField::as_select())
            .load::<ClientField>(&mut conn)?;

        let field_map = fields.into_iter().map(|f| (f.field, f.value)).collect();

        let mut result: Client = client.into();
        result.fields = Some(field_map);

        Ok(Some(result))
    }

    fn get_client_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .filter(clients::email.eq(email))
            .filter(clients::hub_id.eq(hub_id))
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
        let db_clients = items.order(clients::id.asc()).load::<DbClient>(&mut conn)?;
        // .into_iter()
        // .map(Into::into)
        // .collect::<Vec<Client>>();
        if db_clients.is_empty() {
            return Ok((total, Vec::new()));
        }

        // Load recipient fields, grouped by recipient
        let db_fields = ClientField::belonging_to(&db_clients)
            .select(ClientField::as_select())
            .load::<ClientField>(&mut conn)?
            .grouped_by(&db_clients);

        let clients = db_clients
            .into_iter()
            .zip(db_fields)
            .map(|(c, f)| {
                let mut client: Client = c.into();
                let fields = f.into_iter().map(|f| (f.field, f.value)).collect();
                client.fields = Some(fields);
                client
            })
            .collect();

        Ok((total, clients))
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
        use crate::schema::{client_fields, clients};

        let mut conn = self.conn()?;

        conn.transaction::<usize, RepositoryError, _>(|conn| {
            let mut count_inserted: usize = 0;

            for new in new_clients {
                let db_new: DbNewClient = new.into();

                let inserted = diesel::insert_into(clients::table)
                    .values(&db_new)
                    .on_conflict((clients::email, clients::hub_id))
                    .do_update()
                    .set((
                        clients::name.eq(&new.name),
                        clients::phone.eq(&new.phone),
                        clients::address.eq(&new.address),
                    ))
                    .get_result::<DbClient>(conn);

                let client_id = match inserted {
                    Ok(client) => client.id,
                    Err(_) => continue,
                };

                // Insert optional fields
                if let Some(fields) = &new.fields {
                    let new_fields: Vec<ClientField> = fields
                        .iter()
                        .map(|(f, v)| ClientField {
                            client_id: client_id,
                            field: f.clone(),
                            value: v.clone(),
                        })
                        .collect();
                    if !new_fields.is_empty() {
                        for field in new_fields {
                            diesel::insert_into(client_fields::table)
                                .values(&field)
                                .on_conflict((client_fields::client_id, client_fields::field))
                                .do_update()
                                .set(client_fields::value.eq(excluded(client_fields::value)))
                                .execute(conn)?;
                        }
                    }
                }

                count_inserted += 1;
            }

            Ok(count_inserted)
        })
    }

    fn update_client(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client> {
        use crate::schema::{client_fields, clients};

        let mut conn = self.conn()?;
        let db_updates: DbUpdateClient = updates.into();

        let mut updated: Client = diesel::update(clients::table.find(client_id))
            .set(&db_updates)
            .get_result::<DbClient>(&mut conn)?
            .into();

        // Update fields (delete all → insert new)
        diesel::delete(client_fields::table.filter(client_fields::client_id.eq(client_id)))
            .execute(&mut conn)?;
        if let Some(fields) = &updates.fields {
            for (field, value) in fields {
                let new_field = ClientField {
                    client_id,
                    field: field.to_string(),
                    value: value.to_string(),
                };
                diesel::insert_into(client_fields::table)
                    .values(&new_field)
                    .execute(&mut conn)?;
            }
        }

        // Reload fields
        let fields_vec = client_fields::table
            .filter(client_fields::client_id.eq(client_id))
            .select(ClientField::as_select())
            .load::<ClientField>(&mut conn)?;

        let fields_map = fields_vec
            .into_iter()
            .map(|f| (f.field, f.value))
            .collect::<HashMap<_, _>>();

        updated.fields = Some(fields_map);

        Ok(updated)
    }

    fn delete_client(&self, client_id: i32) -> RepositoryResult<()> {
        use crate::schema::{client_fields, client_manager, clients};

        let mut conn = self.conn()?;
        diesel::delete(client_manager::table.filter(client_manager::client_id.eq(client_id)))
            .execute(&mut conn)?;
        diesel::delete(client_fields::table.filter(client_fields::client_id.eq(client_id)))
            .execute(&mut conn)?;
        diesel::delete(clients::table.find(client_id)).execute(&mut conn)?;
        Ok(())
    }
}
