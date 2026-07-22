create table if not exists routes (
    id text not null primary key,
    project_id text not null references projects(id) on delete cascade,
    domain text not null,
    path_prefix text not null default '/',
    desired_state text not null default 'active' check (desired_state in ('active', 'removed')),
    status text not null default 'pending' check (status in ('pending', 'synced', 'failed', 'removed')),
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch()),
    unique(domain, path_prefix)
);

create index if not exists idx_routes_project on routes(project_id);

create table if not exists proxy_settings (
    id integer not null primary key check (id = 1),
    desired_state text not null default 'stopped' check (desired_state in ('running', 'stopped')),
    status text not null default 'stopped' check (status in ('pending', 'starting', 'running', 'failed', 'stopping', 'stopped')),
    dns_provider text check (dns_provider in ('cloudflare')),
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

insert into proxy_settings (id) values (1);

create table if not exists proxy_dns_credentials (
    credential_name text not null primary key,
    parameter_key text not null references parameters(key)
);
