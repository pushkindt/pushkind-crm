// @generated automatically by Diesel CLI.

diesel::table! {
    client_manager (client_id, manager_id) {
        client_id -> Integer,
        manager_id -> Integer,
    }
}

diesel::table! {
    clients (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
        phone -> Text,
        address -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    managers (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
    }
}

diesel::joinable!(client_manager -> clients (client_id));
diesel::joinable!(client_manager -> managers (manager_id));

diesel::allow_tables_to_appear_in_same_query!(
    client_manager,
    clients,
    managers,
);
