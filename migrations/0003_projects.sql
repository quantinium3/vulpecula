create table if not exists projects (
    id text not null primary key,
    name text not null unique,
    source_kind text not null check (source_kind in ('docker_image', 'git_remote_framework', 'git_remote_dockerfile','local_framework', 'local_dockerfile')),
    source_config text not null,
    port integer,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

create table if not exists builds (
    id text not null primary key,
    project_id text not null references projects(id) on delete cascade,
    status text not null check (status in ('pending', 'building', 'success', 'failed')),
    sha text,
    image_ref text,
    log_path text,
    started_at integer not null default (unixepoch()),
    finished_at integer
);

create index if not exists idx_builds_project on builds(project_id);
