use diesel::prelude::*;

use crate::{
    db::DbPool,
    domain::client::{Client, NewClient, UpdateClient},
    pagination::Paginated,
    repository::{ClientRepository, errors::RepositoryResult},
};

/// Diesel implementation of [`ClientRepository`].
pub struct DieselClientRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselClientRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl ClientRepository for DieselClientRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Client>> {
        use crate::models::client::Client as DbClient;
        use crate::schema::clients;

        let mut conn = self.pool.get()?;
        let client = clients::table
            .find(id)
            .first::<DbClient>(&mut conn)
            .optional()?;

        Ok(client.map(Into::into))
    }

    fn create(&self, new_clients: &[NewClient]) -> RepositoryResult<usize> {
        use crate::models::client::NewClient as DbNewClient;
        use crate::schema::clients;

        let mut conn = self.pool.get()?;
        let insertables: Vec<DbNewClient> = new_clients.iter().map(|c| c.into()).collect();
        let affected = diesel::insert_into(clients::table)
            .values(&insertables)
            .execute(&mut conn)?;

        Ok(affected)
    }

    fn list(&self, hub_id: i32, current_page: usize) -> RepositoryResult<Paginated<Client>> {
        use crate::models::client::Client as DbClient;
        use crate::schema::clients;

        let mut conn = self.pool.get()?;
        let per_page: i64 = 20;
        let page = if current_page == 0 { 1 } else { current_page } as i64;
        let offset = (page - 1) * per_page;

        let items = clients::table
            .filter(clients::hub_id.eq(hub_id))
            .order(clients::id.asc())
            .limit(per_page)
            .offset(offset)
            .load::<DbClient>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Client>>();

        let total: i64 = clients::table
            .filter(clients::hub_id.eq(hub_id))
            .count()
            .get_result(&mut conn)?;

        let total_pages = ((total + per_page - 1) / per_page) as usize;

        Ok(Paginated::new(items, page as usize, total_pages))
    }

    fn list_by_manager(
        &self,
        manager_email: &str,
        hub_id: i32,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
        use crate::models::client::Client as DbClient;
        use crate::schema::{client_manager, clients, managers};

        let mut conn = self.pool.get()?;
        let per_page: i64 = 20;
        let page = if current_page == 0 { 1 } else { current_page } as i64;
        let offset = (page - 1) * per_page;

        let manager_id = managers::table
            .filter(managers::email.eq(manager_email))
            .filter(managers::hub_id.eq(hub_id))
            .select(managers::id);

        let items = clients::table
            .inner_join(client_manager::table.on(client_manager::client_id.eq(clients::id)))
            .filter(client_manager::manager_id.eq_any(manager_id))
            .order(clients::id.asc())
            .limit(per_page)
            .offset(offset)
            .select(clients::all_columns)
            .load::<DbClient>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Client>>();

        let total: i64 = clients::table
            .inner_join(client_manager::table)
            .filter(client_manager::manager_id.eq_any(manager_id))
            .count()
            .get_result(&mut conn)?;

        let total_pages = ((total + per_page - 1) / per_page) as usize;

        Ok(Paginated::new(items, page as usize, total_pages))
    }

    fn search(
        &self,
        hub_id: i32,
        search_key: &str,
        current_page: usize,
    ) -> RepositoryResult<Paginated<Client>> {
        use crate::models::client::Client as DbClient;
        use crate::schema::clients;

        let mut conn = self.pool.get()?;
        let per_page: i64 = 20;
        let page = if current_page == 0 { 1 } else { current_page } as i64;
        let offset = (page - 1) * per_page;

        let pattern = format!("%{search_key}%");

        let items = clients::table
            .filter(clients::hub_id.eq(hub_id))
            .filter(
                clients::name
                    .like(&pattern)
                    .or(clients::email.like(&pattern))
                    .or(clients::phone.like(&pattern))
                    .or(clients::address.like(&pattern)),
            )
            .order(clients::id.asc())
            .limit(per_page)
            .offset(offset)
            .load::<DbClient>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Client>>();

        let total: i64 = clients::table
            .filter(clients::hub_id.eq(hub_id))
            .filter(
                clients::name
                    .like(&pattern)
                    .or(clients::email.like(&pattern))
                    .or(clients::phone.like(&pattern))
                    .or(clients::address.like(&pattern)),
            )
            .count()
            .get_result(&mut conn)?;

        let total_pages = ((total + per_page - 1) / per_page) as usize;

        Ok(Paginated::new(items, page as usize, total_pages))
    }

    fn update(&self, client_id: i32, updates: &UpdateClient) -> RepositoryResult<Client> {
        use crate::models::client::{Client as DbClient, UpdateClient as DbUpdateClient};
        use crate::schema::clients;

        let mut conn = self.pool.get()?;
        let db_updates: DbUpdateClient = updates.into();

        let updated = diesel::update(clients::table.find(client_id))
            .set(&db_updates)
            .get_result::<DbClient>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete(&self, client_id: i32) -> RepositoryResult<()> {
        use crate::schema::client_manager;
        use crate::schema::clients;

        let mut conn = self.pool.get()?;

        diesel::delete(client_manager::table.filter(client_manager::client_id.eq(client_id)))
            .execute(&mut conn)?;
        diesel::delete(clients::table.find(client_id)).execute(&mut conn)?;
        Ok(())
    }
}
