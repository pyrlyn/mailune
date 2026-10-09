//! Diesel table definitions for schema v1. They mirror
//! `migrations/*/up.sql` by hand; a test runs the migrations and touches
//! every table through these definitions, so a drift fails there.

diesel::table! {
    accounts (id) {
        id -> Text,
        email -> Text,
        display_name -> Nullable<Text>,
        provider -> Text,
    }
}

diesel::table! {
    mailboxes (account_id, id) {
        account_id -> Text,
        id -> Text,
        name -> Text,
        role -> Nullable<Text>,
        parent_id -> Nullable<Text>,
    }
}

diesel::table! {
    threads (account_id, id) {
        account_id -> Text,
        id -> Text,
        subject -> Text,
        latest_at -> BigInt,
        message_count -> Integer,
        unread_count -> Integer,
    }
}

diesel::table! {
    messages (account_id, id) {
        account_id -> Text,
        id -> Text,
        thread_id -> Text,
        from_name -> Nullable<Text>,
        from_email -> Text,
        to_addrs -> Text,
        cc_addrs -> Text,
        subject -> Text,
        snippet -> Text,
        stamp -> Text,
        received_at -> BigInt,
        attachment_count -> Integer,
        transport -> Text,
        seen -> Bool,
        flagged -> Bool,
        draft -> Bool,
        answered -> Bool,
        deleted -> Bool,
        body_hash -> Nullable<Text>,
    }
}

diesel::table! {
    memberships (account_id, message_id, mailbox_id) {
        account_id -> Text,
        message_id -> Text,
        mailbox_id -> Text,
        uid -> Nullable<BigInt>,
    }
}

diesel::table! {
    parts (account_id, message_id, part_id) {
        account_id -> Text,
        message_id -> Text,
        part_id -> Text,
        content_type -> Text,
        filename -> Nullable<Text>,
        size -> BigInt,
        blob_hash -> Nullable<Text>,
    }
}

diesel::table! {
    flags (account_id, message_id, keyword) {
        account_id -> Text,
        message_id -> Text,
        keyword -> Text,
    }
}

diesel::table! {
    sync_state (account_id, scope) {
        account_id -> Text,
        scope -> Text,
        state -> Text,
    }
}

diesel::table! {
    ops (seq) {
        seq -> Integer,
        op_key -> Text,
        kind -> Text,
        thread_id -> Text,
        from_mailbox -> Nullable<Text>,
        to_mailbox -> Nullable<Text>,
        wake_ns -> Nullable<BigInt>,
        queued_ns -> BigInt,
    }
}

diesel::table! {
    contacts (account_id, email) {
        account_id -> Text,
        email -> Text,
        name -> Nullable<Text>,
        last_seen -> BigInt,
        times_contacted -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    mailboxes,
    threads,
    messages,
    memberships,
    parts,
    flags,
    sync_state,
    ops,
    contacts,
);

diesel::table! {
    blobs (hash) {
        hash -> Text,
        data -> Binary,
        size -> BigInt,
        used -> BigInt,
    }
}

diesel::table! {
    embeddings (account_id, message_id, chunk, model) {
        account_id -> Text,
        message_id -> Text,
        chunk -> Integer,
        model -> Text,
        dim -> Integer,
        vector -> Binary,
    }
}
