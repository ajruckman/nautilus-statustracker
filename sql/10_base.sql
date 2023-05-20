create table nautilus_update
(
    id             int         not null generated always as identity,
    current_status text,
    ship_location  text,
    update_message text,
    update_time    text,
    fetched_at     timestamptz not null default now(),

    constraint nautilus_update_pk primary key (id)
);
