create table if not exists parameters (
    key text not null primary key check (
        key like '/%' and key not like '%//%' and key not like '%/'
    ),
    type text not null check (type in ('string', 'secure_string')),
    value text,
    ciphertext blob,
    nonce blob,
    wrapped_dek blob,
    dek_nonce blob,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch()),
    check (
        (type = 'string' and value is not null and ciphertext is null)
        or
        (type = 'secure_string' and value is null and ciphertext is not null
            and nonce is not null and wrapped_dek is not null and dek_nonce is not null)
    )
);
