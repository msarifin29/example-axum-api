create table users(
    user_id varchar(50) primary key,
    user_name varchar(50) not null unique,
    email varchar(50) not null,
    password varchar(100) not null,
    created_at timestamp not null default current_timestamp,
    updated_at timestamp null default null 
);