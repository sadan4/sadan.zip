use macros::command;

mod ping;

#[command]
#[sub_cmds(ping::ping)]
#[group]
#[root]
struct Root;
