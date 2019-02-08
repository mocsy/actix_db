table! {
    user_sessions (id) {
        id -> Int4,
        skey -> Text,
        sdata -> Nullable<Jsonb>,
    }
}
