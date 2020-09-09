use mux_stream::{demux, panicking};

use derive_more::From;
use futures::{Stream, StreamExt};
use tokio::stream;

#[derive(Debug, From)]
enum AdminUpdate {
    RegisterUser(RegisterUserUpdate),
    DeleteUser(DeleteUserUpdate),
    PinMessage(PinMessageUpdate),
}

#[derive(Debug)]
struct RegisterUserUpdate {
    username: String,
    id: i64,
}

#[derive(Debug)]
struct DeleteUserUpdate {
    id: i64,
}

#[derive(Debug)]
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

    let (register_users_updates, delete_users_updates, pin_messages_updates) =
        demux!(AdminUpdate::RegisterUser, AdminUpdate::DeleteUser, AdminUpdate::PinMessage)(
            panicking(),
        )(updates.boxed());

    tokio::join!(
        register_users(register_users_updates),
        delete_users(delete_users_updates),
        pin_messages(pin_messages_updates)
    );
}

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