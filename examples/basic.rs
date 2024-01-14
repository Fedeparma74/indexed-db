use anyhow::Context;
use indexed_db::Factory;
use web_sys::js_sys::JsString;

async fn example() -> anyhow::Result<()> {
    // Obtain the database builder
    let factory = Factory::get().context("opening IndexedDB")?;

    // Open the database, creating it if needed
    let db = factory
        .open("database", 1, |evt| {
            let db = evt.database();
            db.build_object_store("store").auto_increment().create()?;
            Ok(())
        })
        .await
        .context("creating the 'database' IndexedDB")?;

    // In a transaction, add two records
    db.transaction(&["store"])
        .rw()
        .run(|t| async move {
            let store = t.object_store("store")?;
            store.add(&JsString::from("foo")).await?;
            store.add(&JsString::from("bar")).await?;
            // The below type specification is due to us not having any
            // user-defined error. In these circumstances, we need to
            // explicitly indicate the error type to the Rust type checker.
            Ok::<_, indexed_db::Error>(())
        })
        .await?;

    // In another transaction, read the first record
    db.transaction(&["store"])
        .run(|t| async move {
            let data = t.object_store("store")?.get_all(Some(1)).await?;
            if data.len() != 1 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unexpected data length",
                ))?;
            }
            // Now that we return a custom error type above, we no longer need
            // the explicit type specification
            Ok(())
        })
        .await?;

    // If we return `Err` from a transaction, then it will abort
    db.transaction(&["store"])
        .rw()
        .run(|t| async move {
            let store = t.object_store("store")?;
            store.add(&JsString::from("baz")).await?;
            if store.count().await? > 2 {
                // Oops! In this example, we have 3 items by this point
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Too many objects in store",
                ))?;
            }
            Ok(())
        })
        .await
        .unwrap_err();

    // And no write will have happened
    db.transaction(&["store"])
        .run(|t| async move {
            let num_items = t.object_store("store")?.count().await?;
            assert_eq!(num_items, 2);
            Ok::<_, indexed_db::Error>(())
        })
        .await?;

    Ok(())
}

use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
async fn test() {
    example().await.unwrap()
}

fn main() {}
