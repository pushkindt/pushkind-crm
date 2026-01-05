// @generated automatically by Diesel CLI.

diesel::table! {
    client_events (id) {
        id -> Integer,
        client_id -> Integer,
        manager_id -> Integer,
        event_type -> Text,
        event_data -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    client_fields (client_id, field) {
        client_id -> Integer,
        field -> Text,
        value -> Text,
    }
}

diesel::table! {
    client_fts (rowid) {
        rowid -> Integer,
        name -> Nullable<Binary>,
        email -> Nullable<Binary>,
        phone -> Nullable<Binary>,
        fields -> Nullable<Binary>,
        #[sql_name = "client_fts"]
        client_fts_col -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    client_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    client_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    client_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    client_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
    }
}

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
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        fields -> Nullable<Text>,
        public_id -> Nullable<Binary>,
    }
}

diesel::table! {
    important_fields (hub_id, field) {
        hub_id -> Integer,
        field -> Text,
    }
}

diesel::table! {
    managers (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
        is_user -> Bool,
    }
}

diesel::joinable!(client_events -> clients (client_id));
diesel::joinable!(client_events -> managers (manager_id));
diesel::joinable!(client_fields -> clients (client_id));
diesel::joinable!(client_manager -> clients (client_id));
diesel::joinable!(client_manager -> managers (manager_id));

diesel::allow_tables_to_appear_in_same_query!(
    client_events,
    client_fields,
    client_fts,
    client_fts_config,
    client_fts_data,
    client_fts_docsize,
    client_fts_idx,
    client_manager,
    clients,
    important_fields,
    managers,
);
