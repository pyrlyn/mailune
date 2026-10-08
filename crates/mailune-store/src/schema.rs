//! Diesel tables for the v1 migration.
//!
//! Column modules come from the `table!` macro, which does not emit docs.
//! FTS5 virtual tables cannot be expressed in Diesel's DSL, so `message_fts`
//! is created in a migration and queried with `sql_query`.

#![allow(missing_docs)]

diesel::table! {
    accounts (id) {
        id -> Text,
        email -> Text,
    }
}

diesel::table! {
    mailboxes (id) {
        id -> Text,
        account_id -> Text,
        name -> Text,
        role -> Nullable<Text>,
    }
}

diesel::table! {
    threads (id) {
        id -> Text,
        account_id -> Text,
        subject -> Text,
    }
}

diesel::table! {
    messages (id) {
        id -> Text,
        account_id -> Text,
        thread_id -> Text,
        subject -> Text,
        from_email -> Text,
        to_emails -> Text,
        stamp -> Text,
        received_at -> BigInt,
    }
}

diesel::table! {
    memberships (message_id, mailbox_id) {
        message_id -> Text,
        mailbox_id -> Text,
    }
}

diesel::table! {
    parts (message_id, ordinal) {
        message_id -> Text,
        ordinal -> Integer,
        content_type -> Text,
        body -> Text,
    }
}

diesel::table! {
    flags (message_id) {
        message_id -> Text,
        seen -> Bool,
        flagged -> Bool,
        draft -> Bool,
        answered -> Bool,
        deleted -> Bool,
        keywords -> Text,
    }
}

diesel::table! {
    sync_state (account_id, mailbox_id) {
        account_id -> Text,
        mailbox_id -> Text,
        token -> Text,
    }
}

diesel::table! {
    ops (key) {
        key -> Text,
        kind -> Text,
        payload -> Text,
        at_ms -> BigInt,
        position -> Integer,
    }
}

diesel::table! {
    contacts (account_id, email) {
        account_id -> Text,
        email -> Text,
        name -> Text,
    }
}
