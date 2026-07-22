create table if not exists projects (
    id text not null primary key,
    name text not null unique,
    source_kind text not null check (source_kind in ('docker_image', 'git_repo', 'local_repo')),
    image text,
    repo_url text,
    branch text,
    framework text check (framework in ('dockerfile', 'react', 'svelte', 'express', 'static')),
    root_dir text,
    install_command text,
    build_command text,
    output_directory text,
    start_command text,
    port integer,
    container_port integer,
    retention_count integer not null default 3,
    created_at integer not null default (unixepoch()),
    updated_at integer not null default (unixepoch())
);

create table if not exists project_env (
    id integer primary key,
    project_id text not null references projects(id) on delete cascade,
    env_name text not null,
    parameter_key text not null,
    unique(project_id, env_name)
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
