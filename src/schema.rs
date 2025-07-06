// @generated automatically by Diesel CLI.

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
