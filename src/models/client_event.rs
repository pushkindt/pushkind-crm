//! Diesel models for storing CRM client events.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::client_event::{
    ClientEvent as DomainClientEvent, ClientEventType, NewClientEvent as DomainNewClientEvent,
};
use crate::models::client::Client;
use crate::models::manager::Manager;

#[derive(Debug, Clone, Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Client, foreign_key=client_id))]
#[diesel(belongs_to(Manager, foreign_key=manager_id))]
#[diesel(table_name = crate::schema::client_events)]
pub struct ClientEvent {
    pub id: i32,
    pub client_id: i32,
    pub manager_id: i32,
    pub event_type: String,
    pub event_data: String, // store JSON text in the DB
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::client_events)]
pub struct NewClientEvent {
    pub client_id: i32,
    pub manager_id: i32,
    pub event_type: String,
    pub event_data: String,
}

impl From<ClientEvent> for DomainClientEvent {
    fn from(event: ClientEvent) -> Self {
        Self {
            id: event.id,
            client_id: event.client_id,
            manager_id: event.manager_id,
            event_type: ClientEventType::from(event.event_type.as_str()),
            event_data: serde_json::from_str(&event.event_data).unwrap_or_default(),
            created_at: event.created_at,
        }
    }
}

impl<'a> From<&'a DomainNewClientEvent> for NewClientEvent {
    fn from(event: &'a DomainNewClientEvent) -> Self {
        Self {
            client_id: event.client_id,
            manager_id: event.manager_id,
            event_type: event.event_type.to_string(),
            event_data: event.event_data.to_string(),
        }
    }
}

impl From<DomainNewClientEvent> for NewClientEvent {
    fn from(event: DomainNewClientEvent) -> Self {
        Self::from(&event)
    }
}
