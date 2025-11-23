create table groups(
    group_id varchar(50) primary key,
    name varchar(50) not null unique,
    description text,
    created_at timestamp not null default current_timestamp,
    updated_at timestamp null default null 
);