use diesel::prelude::*;
use pushkind_common::db::DbPool;

use crate::{
    domain::{
        client::Client,
        manager::{Manager, NewManager},
    },
    models::{
        client::Client as DbClient,
        manager::{
            Manager as DbManager, NewClientManager as DbNewClientManager,
            NewManager as DbNewManager, UpdateManager as DbUpdateManager,
        },
    },
    repository::{ManagerReader, ManagerWriter, errors::RepositoryResult},
};

/// Diesel implementation of [`ManagerRepository`].
pub struct DieselManagerRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselManagerRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl ManagerWriter for DieselManagerRepository<'_> {
    fn create_or_update(&self, new_manager: &NewManager) -> RepositoryResult<Manager> {
        use crate::schema::managers;

        let mut conn = self.pool.get()?;

        let db_new_manager: DbNewManager = new_manager.into();

        let db_update_manager: DbUpdateManager = new_manager.into();

        let db_manager = diesel::insert_into(managers::table)
            .values(&db_new_manager)
            .on_conflict((managers::email, managers::hub_id))
            .do_update()
            .set(&db_update_manager)
            .get_result::<DbManager>(&mut conn)?;

        Ok(db_manager.into())
    }

    fn assign_clients(&self, manager_id: i32, client_ids: &[i32]) -> RepositoryResult<usize> {
        use crate::schema::client_manager;

        let mut conn = self.pool.get()?;

        let db_client_manager = client_ids
            .iter()
            .map(|client_id| DbNewClientManager {
                client_id: *client_id,
                manager_id,
            })
            .collect::<Vec<_>>();

        diesel::delete(client_manager::table.filter(client_manager::manager_id.eq(manager_id)))
            .execute(&mut conn)?;

        let result = diesel::insert_into(client_manager::table)
            .values(db_client_manager)
            .execute(&mut conn)?;

        Ok(result)
    }
}

impl ManagerReader for DieselManagerRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Manager>> {
        use crate::schema::managers;

        let mut conn = self.pool.get()?;
        let db_manager = managers::table
            .filter(managers::id.eq(id))
            .first::<DbManager>(&mut conn)
            .optional()?;

        Ok(db_manager.map(|db_manager| db_manager.into()))
    }

    fn get_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<Manager>> {
        use crate::schema::managers;

        let mut conn = self.pool.get()?;
        let db_manager = managers::table
            .filter(managers::email.eq(email))
            .filter(managers::hub_id.eq(hub_id))
            .first::<DbManager>(&mut conn)
            .optional()?;

        Ok(db_manager.map(|db_manager| db_manager.into()))
    }

    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<(Manager, Vec<Client>)>> {
        use crate::schema::client_manager;
        use crate::schema::clients;
        use crate::schema::managers;

        let mut conn = self.pool.get()?;
        let managers = managers::table
            .filter(managers::hub_id.eq(hub_id))
            .load::<DbManager>(&mut conn)?;

        let managers_ids = managers
            .iter()
            .map(|db_manager| db_manager.id)
            .collect::<Vec<i32>>();

        let clients = clients::table
            .inner_join(client_manager::table)
            .filter(client_manager::manager_id.eq_any(managers_ids))
            .select((client_manager::manager_id, clients::all_columns))
            .load::<(i32, DbClient)>(&mut conn)?;

        let manager_with_clients = managers
            .into_iter()
            .map(|manager| {
                let manager_clients = clients
                    .iter()
                    .filter(|(manager_id, _)| *manager_id == manager.id)
                    .map(|(_, client)| client.clone().into())
                    .collect();
                (manager.into(), manager_clients)
            })
            .collect();

        Ok(manager_with_clients) // Convert DbUser to DomainUser
    }
}
