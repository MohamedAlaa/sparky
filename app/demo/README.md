# Demo mailbox

`sparky.sqlite3` is a clean database holding only Sparky's eight sample
conversations — no real mail, accounts or tokens. Try the app with it:

    cd app
    cp -R demo /tmp/sparky-demo        # keep the committed copy pristine
    SPARKY_DATA_DIR=/tmp/sparky-demo cargo run --release

`SPARKY_DATA_DIR` points Sparky at any folder; an empty folder is seeded with
the same demo mail on first launch. Your own mailbox (`sparky.sqlite3` next to
`Cargo.toml` in debug builds, `~/Library/Application Support/Sparky/` in
release builds) is ignored by git and never committed.

To regenerate it after a schema change:

    d=$(mktemp -d); SPARKY_DATA_DIR=$d cargo run --release -- --screenshot $d/x.png
    sqlite3 $d/sparky.sqlite3 "PRAGMA journal_mode=DELETE; VACUUM;"
    cp $d/sparky.sqlite3 demo/sparky.sqlite3
