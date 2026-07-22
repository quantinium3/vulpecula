create table if not exists containers (
    id text not null primary key,
    project_id text not null unique references projects(id) on delete cascade,
    name text not null unique,
    docker_container_id text,
    pending_docker_container_id text,
    desired_state text not null default 'stopped' check (desired_state in ('running', 'stopped', 'removed')),
    status text not null default 'stopped' check (status in ('pending', 'creating', 'running', 'failed', 'stopping', 'stopped', 'removing', 'removed', 'cutting_over')),
    current_revision integer not null default 0,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

create table if not exists container_revisions (
    id integer primary key,
    container_id text not null references containers(id) on delete cascade,
    revision integer not null,
    build_id text references builds(id),
    spec_json text not null,
    created_at integer not null default (unixepoch()),
    unique(container_id, revision)
);

create table if not exists container_state_transitions (
    id integer primary key,
    container_id text not null references containers(id) on delete cascade,
    from_status text,
    to_status text not null,
    reason text,
    created_at integer not null default (unixepoch())
);

create index if not exists idx_container_revisions_container on container_revisions(container_id);
create index if not exists idx_container_transitions_container on container_state_transitions(container_id);
