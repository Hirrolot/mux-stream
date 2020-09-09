use mux_stream::{demux, panicking};

use derive_more::From;
use futures::{Stream, StreamExt};
use tokio::stream;

#[derive(From)]
enum AdminUpdate {
    RegisterUser(RegisterUserUpdate),
    DeleteUser(DeleteUserUpdate),
    PinMessage(PinMessageUpdate),
}

struct RegisterUserUpdate {
    username: String,
    id: i64,
}

struct DeleteUserUpdate {
    id: i64,
}

struct PinMessageUpdate {
    message: String,
}

#[tokio::main]
async fn main() {
    let updates = stream::iter(vec![
        RegisterUserUpdate { username: "Sergey".to_owned(), id: 1414 }.into(),
        RegisterUserUpdate { username: "Ivan".to_owned(), id: 22 }.into(),
        PinMessageUpdate { message: "Hello everyone!".to_owned() }.into(),
        DeleteUserUpdate { id: 1414 }.into(),
    ]);

    use AdminUpdate::*;
    let updates = demux!(RegisterUser, DeleteUser, PinMessage)(panicking())(updates.boxed());

    tokio::join!(register_users(updates.0), delete_users(updates.1), pin_messages(updates.2));
}

// There is exactly one processor for each update kind, reflecting the
// single-responsibility principle (SRP):
// https://en.wikipedia.org/wiki/Single-responsibility_principle

async fn register_users<S>(updates: S)
where
    S: Stream<Item = RegisterUserUpdate>,
{
    updates
        .for_each_concurrent(None, |update| async move {
            println!("Registering user #{} '{}'...", update.id, update.username);
        })
        .await;
}

async fn delete_users<S>(updates: S)
where
    S: Stream<Item = DeleteUserUpdate>,
{
    updates
        .for_each_concurrent(None, |update| async move {
            println!("Deleting user #{}...", update.id);
        })
        .await;
}

async fn pin_messages<S>(updates: S)
where
    S: Stream<Item = PinMessageUpdate>,
{
    updates
        .for_each_concurrent(None, |update| async move {
            println!("Pinning message '{}'...", update.message);
        })
        .await;
}
