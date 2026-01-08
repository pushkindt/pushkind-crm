//! Repository implementation handling CRM clients.

use std::collections::BTreeMap;

use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel::sql_types::{Bool, Nullable, Text};
use diesel::upsert::excluded;
use pushkind_common::repository::build_fts_match_query;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::important_field::ImportantField as DomainImportantField;
use crate::domain::types::{
    ClientEmail, ClientId, HubId, ManagerEmail, PublicId, TypeConstraintError,
};
use crate::models::client::ClientField;
use crate::models::important_field::{
    ImportantField as DbImportantField, NewImportantField as DbNewImportantField,
};
use crate::{
    domain::client::{Client, NewClient, UpdateClient},
    domain::manager::Manager,
    models::client::{
        Client as DbClient, NewClient as DbNewClient, UpdateClient as DbUpdateClient,
    },
    models::manager::Manager as DbManager,
    repository::{
        ClientListQuery, ClientReader, ClientWriter, DieselRepository, ImportantFieldReader,
        ImportantFieldWriter,
    },
};

impl ClientReader for DieselRepository {
    fn get_client_by_public_id(
        &self,
        public_id: PublicId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .filter(clients::public_id.eq(public_id.as_bytes()))
            .filter(clients::hub_id.eq(hub_id.get()))
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

        let mut result: Client = Client::try_from(client).map_err(RepositoryError::from)?;
        result.fields = Some(field_map);

        Ok(Some(result))
    }

    fn list_available_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<String>> {
        use crate::schema::{client_fields, clients, important_fields};

        let mut conn = self.conn()?;

        let mut fields = client_fields::table
            .inner_join(clients::table)
            .filter(clients::hub_id.eq(hub_id.get()))
            .select(client_fields::field)
            .distinct()
            .load::<String>(&mut conn)?;

        let important = important_fields::table
            .filter(important_fields::hub_id.eq(hub_id.get()))
            .select(important_fields::field)
            .load::<String>(&mut conn)?;

        fields.extend(important);
        fields.sort();
        fields.dedup();

        Ok(fields)
    }

    fn get_client_by_id(&self, id: ClientId, hub_id: HubId) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .find(id.get())
            .filter(clients::hub_id.eq(hub_id.get()))
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

        let mut result: Client = Client::try_from(client).map_err(RepositoryError::from)?;
        result.fields = Some(field_map);

        Ok(Some(result))
    }

    fn get_client_by_email(
        &self,
        email: &ClientEmail,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Client>> {
        use crate::schema::clients;

        let mut conn = self.conn()?;
        let client = clients::table
            .filter(clients::email.eq(email.as_str()))
            .filter(clients::hub_id.eq(hub_id.get()))
            .first::<DbClient>(&mut conn)
            .optional()?;

        let client = match client {
            Some(client) => Some(Client::try_from(client).map_err(RepositoryError::from)?),
            None => None,
        };

        Ok(client)
    }

    fn list_clients(&self, query: ClientListQuery) -> RepositoryResult<(usize, Vec<Client>)> {
        use crate::schema::{client_fts, client_manager, clients, managers};

        let mut conn = self.conn()?;

        let query_builder = || {
            // Start with boxed query on clients
            let mut items = clients::table
                .filter(clients::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(public_id) = &query.public_id {
                items = items.filter(clients::public_id.eq(public_id.as_bytes()))
            }

            if let Some(manager_email) = &query.manager_email {
                items = items.filter(
                    clients::id.eq_any(
                        client_manager::table
                            .filter(
                                client_manager::manager_id.nullable().eq(managers::table
                                    .filter(managers::email.eq(manager_email.as_str()))
                                    .filter(managers::hub_id.eq(query.hub_id.get()))
                                    .select(managers::id)
                                    .single_value()),
                            )
                            .select(client_manager::client_id),
                    ),
                );
            }

            if let Some(term) = query.search.as_ref()
                && let Some(fts_query) = build_fts_match_query(term)
            {
                let fts_filter = exists(
                    client_fts::table
                        .filter(client_fts::rowid.eq(clients::id))
                        .filter(
                            diesel::dsl::sql::<Bool>("client_fts MATCH ")
                                .bind::<Text, _>(fts_query),
                        ),
                );
                items = items.filter(fts_filter);
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
                let mut client = Client::try_from(c).map_err(RepositoryError::from)?;
                let fields = f.into_iter().map(|f| (f.field, f.value)).collect();
                client.fields = Some(fields);
                Ok(client)
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;

        Ok((total, clients))
    }

    fn list_managers(&self, id: ClientId) -> RepositoryResult<Vec<Manager>> {
        use crate::schema::{client_manager, clients, managers};
        let mut conn = self.conn()?;
        let client_hub_id = clients::table
            .filter(clients::id.eq(id.get()))
            .select(clients::hub_id)
            .single_value();
        let managers = client_manager::table
            .filter(client_manager::client_id.eq(id.get()))
            .inner_join(managers::table)
            .filter(managers::hub_id.nullable().eq(client_hub_id))
            .select(managers::all_columns)
            .load::<DbManager>(&mut conn)?
            .into_iter()
            .map(|db_manager| Manager::try_from(db_manager).map_err(RepositoryError::from))
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        Ok(managers)
    }

    fn check_client_assigned_to_manager(
        &self,
        client_id: ClientId,
        manager_email: &ManagerEmail,
    ) -> RepositoryResult<bool> {
        use crate::schema::{client_manager, clients, managers};
        let mut conn = self.conn()?;

        let assigned = client_manager::table
            .filter(client_manager::client_id.eq(client_id.get()))
            .inner_join(managers::table)
            .filter(managers::email.eq(manager_email.as_str()))
            .inner_join(clients::table)
            .filter(clients::id.eq(client_id.get()))
            .filter(clients::hub_id.eq(managers::hub_id))
            .select(client_manager::client_id)
            .first::<i32>(&mut conn)
            .optional()?;
        Ok(assigned.is_some())
    }
}

impl ClientWriter for DieselRepository {
    fn create_or_replace_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
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
                        clients::name.eq(new.name.as_str()),
                        clients::email.eq(new.email.as_ref().map(|email| email.as_str())),
                        clients::phone.eq(new.phone.as_ref().map(|phone| phone.as_str())),
                    ))
                    .get_result::<DbClient>(conn);

                let client_id = match inserted {
                    Ok(client) => client.id,
                    Err(err) => {
                        if let diesel::result::Error::DatabaseError(
                            DatabaseErrorKind::UniqueViolation,
                            _,
                        ) = err
                        {
                            // likely conflict on (hub_id, phone), try to find and update existing record
                            let Some(phone) = &new.phone else { continue };

                            let existing = match clients::table
                                .filter(clients::hub_id.eq(new.hub_id.get()))
                                .filter(clients::phone.eq(phone.as_str()))
                                .first::<DbClient>(conn)
                            {
                                Ok(client) => client,
                                Err(_) => continue,
                            };

                            if diesel::update(clients::table.find(existing.id))
                                .set((
                                    clients::name.eq(new.name.as_str()),
                                    clients::email
                                        .eq(new.email.as_ref().map(|email| email.as_str())),
                                    clients::phone
                                        .eq(new.phone.as_ref().map(|phone| phone.as_str())),
                                ))
                                .execute(conn)
                                .is_err()
                            {
                                continue;
                            }

                            existing.id
                        } else {
                            continue;
                        }
                    }
                };

                // Update fields (delete all → insert new)
                diesel::delete(client_fields::table.filter(client_fields::client_id.eq(client_id)))
                    .execute(conn)?;

                // Insert optional fields
                if let Some(fields) = &new.fields {
                    let new_fields: Vec<ClientField> = fields
                        .iter()
                        .map(|(f, v)| ClientField {
                            client_id,
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

                // Update denormalized `clients.fields` using a Diesel subselect
                diesel::update(clients::table.find(client_id))
                    .set(
                        clients::fields.eq(client_fields::table
                            .filter(client_fields::client_id.eq(client_id))
                            .select(diesel::dsl::sql::<Nullable<Text>>(
                                "trim(COALESCE(group_concat(value, ' '), ''))",
                            ))
                            .single_value()),
                    )
                    .execute(conn)?;

                count_inserted += 1;
            }

            Ok(count_inserted)
        })
    }

    fn create_clients(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
        use crate::schema::{client_fields, clients};

        let mut conn = self.conn()?;

        conn.transaction::<usize, RepositoryError, _>(|conn| {
            let mut count_inserted: usize = 0;

            for new in new_clients {
                let db_new: DbNewClient = new.into();

                let inserted = diesel::insert_into(clients::table)
                    .values(&db_new)
                    .get_result::<DbClient>(conn);

                let client_id = match inserted {
                    Ok(client) => client.id,
                    Err(diesel::result::Error::DatabaseError(
                        DatabaseErrorKind::UniqueViolation,
                        _,
                    )) => {
                        continue;
                    }
                    Err(err) => return Err(RepositoryError::from(err)),
                };

                // Update fields (delete all → insert new)
                diesel::delete(client_fields::table.filter(client_fields::client_id.eq(client_id)))
                    .execute(conn)?;

                // Insert optional fields
                if let Some(fields) = &new.fields {
                    let new_fields: Vec<ClientField> = fields
                        .iter()
                        .map(|(f, v)| ClientField {
                            client_id,
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

                // Update denormalized `clients.fields` using a Diesel subselect
                diesel::update(clients::table.find(client_id))
                    .set(
                        clients::fields.eq(client_fields::table
                            .filter(client_fields::client_id.eq(client_id))
                            .select(diesel::dsl::sql::<Nullable<Text>>(
                                "trim(COALESCE(group_concat(value, ' '), ''))",
                            ))
                            .single_value()),
                    )
                    .execute(conn)?;

                count_inserted += 1;
            }

            Ok(count_inserted)
        })
    }

    fn update_client(
        &self,
        client_id: ClientId,
        updates: &UpdateClient,
    ) -> RepositoryResult<Client> {
        use crate::schema::{client_fields, clients};

        let mut conn = self.conn()?;

        let (updated_record, updated_fields) = conn
            .transaction::<(DbClient, BTreeMap<String, String>), diesel::result::Error, _>(
                |conn| {
                    let db_updates: DbUpdateClient = updates.into();
                    let updated = diesel::update(clients::table.find(client_id.get()))
                        .set(&db_updates)
                        .execute(conn)?;
                    if updated == 0 {
                        return Err(diesel::result::Error::NotFound);
                    }

                    // Update fields (delete all → insert new)
                    diesel::delete(
                        client_fields::table.filter(client_fields::client_id.eq(client_id.get())),
                    )
                    .execute(conn)?;

                    if let Some(fields) = &updates.fields {
                        for (field, value) in fields {
                            let new_field = ClientField {
                                client_id: client_id.get(),
                                field: field.to_string(),
                                value: value.to_string(),
                            };
                            diesel::insert_into(client_fields::table)
                                .values(&new_field)
                                .execute(conn)?;
                        }
                    }

                    // Update denormalized `clients.fields` using a Diesel subselect
                    diesel::update(clients::table.find(client_id.get()))
                        .set(
                            clients::fields.eq(client_fields::table
                                .filter(client_fields::client_id.eq(client_id.get()))
                                .select(diesel::dsl::sql::<Nullable<Text>>(
                                    "trim(COALESCE(group_concat(value, ' '), ''))",
                                ))
                                .single_value()),
                        )
                        .execute(conn)?;

                    // Reload the client row with its fields.
                    let updated_client = clients::table
                        .find(client_id.get())
                        .first::<DbClient>(conn)?;

                    let fields_vec = client_fields::table
                        .filter(client_fields::client_id.eq(client_id.get()))
                        .select(ClientField::as_select())
                        .load::<ClientField>(conn)?;

                    let fields_map = fields_vec
                        .into_iter()
                        .map(|f| (f.field, f.value))
                        .collect::<BTreeMap<_, _>>();

                    Ok((updated_client, fields_map))
                },
            )?;

        let mut updated = Client::try_from(updated_record).map_err(RepositoryError::from)?;
        updated.fields = Some(updated_fields);

        Ok(updated)
    }

    fn delete_client(&self, client_id: ClientId) -> RepositoryResult<()> {
        use crate::schema::{client_events, client_fields, client_manager, clients};

        let mut conn = self.conn()?;

        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            diesel::delete(
                client_events::table.filter(client_events::client_id.eq(client_id.get())),
            )
            .execute(conn)?;
            diesel::delete(
                client_manager::table.filter(client_manager::client_id.eq(client_id.get())),
            )
            .execute(conn)?;
            diesel::delete(
                client_fields::table.filter(client_fields::client_id.eq(client_id.get())),
            )
            .execute(conn)?;
            let deleted = diesel::delete(clients::table.find(client_id.get())).execute(conn)?;
            if deleted == 0 {
                return Err(diesel::result::Error::NotFound);
            }
            Ok(())
        })
        .map_err(RepositoryError::from)
    }
}

impl ImportantFieldReader for DieselRepository {
    fn list_important_fields(&self, hub: HubId) -> RepositoryResult<Vec<DomainImportantField>> {
        use crate::schema::important_fields;

        let mut conn = self.conn()?;

        let rows: Vec<DbImportantField> = important_fields::table
            .filter(important_fields::hub_id.eq(hub.get()))
            .order(important_fields::field.asc())
            .load(&mut conn)?;

        let fields = rows
            .into_iter()
            .map(DomainImportantField::try_from)
            .collect::<Result<Vec<_>, TypeConstraintError>>()
            .map_err(RepositoryError::from)?;

        Ok(fields)
    }
}

impl ImportantFieldWriter for DieselRepository {
    fn replace_important_fields(
        &self,
        hub: HubId,
        fields: &[DomainImportantField],
    ) -> RepositoryResult<()> {
        use crate::schema::important_fields;

        let mut conn = self.conn()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::delete(important_fields::table.filter(important_fields::hub_id.eq(hub.get())))
                .execute(conn)?;

            if fields.is_empty() {
                return Ok(());
            }

            let new_fields: Vec<DbNewImportantField<'_>> =
                fields.iter().map(DbNewImportantField::from).collect();

            diesel::insert_into(important_fields::table)
                .values(&new_fields)
                .execute(conn)?;

            Ok(())
        })
        .map_err(RepositoryError::from)
    }
}
