use macros::command;

mod ping;

#[command]
#[sub_cmds(ping::ping, dev::dev)]
#[group]
#[root]
struct Root;

mod dev;
