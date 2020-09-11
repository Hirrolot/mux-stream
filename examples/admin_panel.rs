use mux_stream::{demux, dispatch, panicking};

use derive_more::From;
use futures::{future, Stream, StreamExt};
use tokio::stream;

// Imagine you're writing an admin panel. Here are the hierarchy of updates.
#[derive(From)]
enum AdminUpdate {
    // The following two actions are possibly performed by another admin.
    UserRegistered(UserRegisteredUpdate),
    UserDeleted(UserDeletedUpdate),

    // An incoming private messages sent to an admin.
    PrivateMessage(PrivateMessageUpdate),
}

struct UserRegisteredUpdate {
    username: String,
    id: i64,
}

struct UserDeletedUpdate {
    id: i64,
}

struct PrivateMessageUpdate {
    message: String,
}

#[tokio::main]
async fn main() {
    // In the real world, you obtain this stream from network.
    let updates = stream::iter(vec![
        UserRegisteredUpdate { username: "Sergey".to_owned(), id: 1414 }.into(),
        UserRegisteredUpdate { username: "Ivan".to_owned(), id: 22 }.into(),
        UserDeletedUpdate { id: 1414 }.into(),
        PrivateMessageUpdate { message: "Dazy has done her project.".to_owned() }.into(),
        // These two spam letters will be filtered:
        PrivateMessageUpdate { message: "Buy our magazine!!!".to_owned() }.into(),
        PrivateMessageUpdate { message: "Learn how to code Python for free!!!".to_owned() }.into(),
    ]);

    // Demultiplex updates into three streams:
    //  1) of UserRegisteredUpdate,
    //  2) of UserDeletedUpdate,
    //  3) of PrivateMessageUpdate.
    //
    // `panicking()` means that if the demultiplexer fails to redirect an element of
    // the input stream into one of the output streams, it'll panic.
    let updates = demux!(AdminUpdate { UserRegistered, UserDeleted, PrivateMessage })(panicking())(
        updates.boxed(),
    );

    // Pass the stream:
    //  1) of UserRegisteredUpdate into process_registered_users
    //  2) of UserDeletedUpdate into process_deleted_users
    //  3) of PrivateMessageUpdate into process_private_messages
    dispatch!(updates => process_registered_users, process_deleted_users, process_private_messages);
}

// There is exactly one processor for each update kind, reflecting the
// single-responsibility principle (SRP):
// https://en.wikipedia.org/wiki/Single-responsibility_principle

async fn process_registered_users<S>(updates: S)
where
    S: Stream<Item = UserRegisteredUpdate>,
{
    updates
        .enumerate()
        .for_each_concurrent(None, |(i, update)| async move {
            println!(
                "User #{} '{}' has been registered ({} users registered at all).",
                update.id,
                update.username,
                i + 1
            );
        })
        .await;
}

async fn process_deleted_users<S>(updates: S)
where
    S: Stream<Item = UserDeletedUpdate>,
{
    updates
        .for_each_concurrent(None, |update| async move {
            println!("User #{} has been deleted.", update.id);
        })
        .await;
}

// Displays incoming private messages, filtering spam.
async fn process_private_messages<S>(updates: S)
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
