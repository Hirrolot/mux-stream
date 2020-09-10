use mux_stream::{demux, dispatch, panicking};

use derive_more::From;
use futures::{future, Stream, StreamExt};
use tokio::stream;

#[derive(From)]
enum AdminUpdate {
    RegisterUser(RegisterUserUpdate),
    DeleteUser(DeleteUserUpdate),
    PrivateMessage(PrivateMessageUpdate),
}

struct RegisterUserUpdate {
    username: String,
    id: i64,
}

struct DeleteUserUpdate {
    id: i64,
}

struct PrivateMessageUpdate {
    message: String,
}

#[tokio::main]
async fn main() {
    let updates = stream::iter(vec![
        RegisterUserUpdate { username: "Sergey".to_owned(), id: 1414 }.into(),
        RegisterUserUpdate { username: "Ivan".to_owned(), id: 22 }.into(),
        DeleteUserUpdate { id: 1414 }.into(),
        PrivateMessageUpdate { message: "Dazy has done her project.".to_owned() }.into(),
        // These two spams will be filtered:
        PrivateMessageUpdate { message: "Buy our magazine!!!".to_owned() }.into(),
        PrivateMessageUpdate { message: "Learn how to code Python for free!!!".to_owned() }.into(),
    ]);

    let updates = demux!(AdminUpdate { RegisterUser, DeleteUser, PrivateMessage })(panicking())(
        updates.boxed(),
    );

    dispatch!(updates => register_users, delete_users, private_messages);
}

// There is exactly one processor for each update kind, reflecting the
// single-responsibility principle (SRP):
// https://en.wikipedia.org/wiki/Single-responsibility_principle

async fn register_users<S>(updates: S)
where
    S: Stream<Item = RegisterUserUpdate>,
{
    updates
        .enumerate()
        .for_each_concurrent(None, |(i, update)| async move {
            println!(
                "Registering user #{} '{}' ({} users at all!)...",
                update.id,
                update.username,
                i + 1
            );
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

async fn private_messages<S>(updates: S)
where
    S: Stream<Item = PrivateMessageUpdate>,
{
    let spam = |update: &PrivateMessageUpdate| {
        future::ready({
            let lowercased = update.message.to_lowercase();
            lowercased.contains("buy")
                || lowercased.contains("for free")
                || lowercased.contains("!!!")
        })
    };

    updates
        .filter(spam)
        .for_each_concurrent(None, |update| async move {
            println!("A private messages has arrived: '{}'.", update.message);
        })
        .await;
}
