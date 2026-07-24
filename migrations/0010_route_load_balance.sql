create table routes_new (
    id text not null primary key,
    project_id text not null references projects(id) on delete cascade,
    domain text not null,
    path_prefix text not null default '/',
    desired_state text not null default 'active' check (desired_state in ('active', 'removed')),
    status text not null default 'pending' check (status in ('pending', 'synced', 'failed', 'removed')),
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch()),
    unique(domain, path_prefix, project_id)
);

insert into routes_new (id, project_id, domain, path_prefix, desired_state, status, created_at, updated_at)
    select id, project_id, domain, path_prefix, desired_state, status, created_at, updated_at from routes;

drop table routes;
alter table routes_new rename to routes;

create index if not exists idx_routes_project on routes(project_id);
