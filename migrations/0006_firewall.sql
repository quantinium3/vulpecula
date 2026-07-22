create table if not exists firewall_settings (
    id integer not null primary key check (id = 1),
    desired_state text not null default 'disabled' check (desired_state in ('enabled', 'disabled')),
    status text not null default 'disabled' check (status in ('pending', 'applying', 'enabled', 'failed', 'disabling', 'disabled')),
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

insert into firewall_settings (id) values (1);
