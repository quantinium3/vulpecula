alter table builds add column number integer not null default 0;
alter table builds add column tag text;
alter table builds add column log text not null default '';
alter table builds add column error text;
alter table builds add column created_at integer not null default (unixepoch());

create index if not exists idx_builds_project_number on builds (project_id, number);
