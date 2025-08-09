E621 Account Parser

A small site for storing personal favorites and generating a personalized post feed.

Tooling instalation
```cmd
cargo install cargo-watch
cargo install --locked trunk
```

Launching backend
```cmd
cd ./parser-api/
cargo watch -x run
```

Launching frontend
```cmd
cd ./parser-web/
trunk serve
```

Backend is launched at localhost:8080
Frontend is launched at localhost:8000