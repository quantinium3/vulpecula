create table if not exists environment_variables (
    key text not null primary key,
    kind text not null check(kind in ('secret', 'variable'))
);

create table if not exists secrets (
    key text not null primary key references environment_variables(key),
    ciphertext blob not null,
    nonce blob not null,
    wrapped_dek blob not null,
    dek_nonce blob not null,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

create table if not exists variables (
    key text not null primary key references environment_variables(key),
    value text not null,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);
