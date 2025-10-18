# E621 Account Parser

A tiny web app for storing personal favorites and generating a personalized post feed.

[![Stars](https://img.shields.io/github/stars/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/stargazers)
[![Watchers](https://img.shields.io/github/watchers/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/watchers)
[![Forks](https://img.shields.io/github/forks/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/network/members)
[![Issues](https://img.shields.io/github/issues/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/issues)
[![Open PRs](https://img.shields.io/github/issues-pr/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/pulls)
[![Contributors](https://img.shields.io/github/contributors/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/graphs/contributors)
[![License](https://img.shields.io/github/license/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/E621-Account-Parser/blob/master/LICENCE)
[![Last Commit](https://img.shields.io/github/last-commit/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/commits)
[![Commit Activity](https://img.shields.io/github/commit-activity/m/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/pulse)
[![Top Language](https://img.shields.io/github/languages/top/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser)
[![Code Size](https://img.shields.io/github/languages/code-size/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser)
[![Repo Size](https://img.shields.io/github/repo-size/Basedfloppa/e621-Account-Parser?style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser)
[![Latest Release](https://img.shields.io/github/v/release/Basedfloppa/e621-Account-Parser?display_name=tag&sort=semver&style=flat-square)](https://github.com/Basedfloppa/e621-Account-Parser/releases)

## ✨ Features
- Save and manage personal favorites
- Generate a customized feed based on your preferences
- Simple local dev setup (Rust backend + Trunk-served frontend)

---

## 🧰 Tooling Installation
Make sure you have [Rust](https://www.rust-lang.org/tools/install) and `cargo` installed. Then:

```bash
cargo install cargo-watch
cargo install --locked trunk
```
>cargo-watch enables hot-reload for the backend, and trunk serves/builds the frontend.

# 🚀 Running Locally

---

## Backend

./config.toml 
```toml
admin_user = "username"
admin_api = "api_key"
tag_blacklist = ["tag1", "tag2", "tagN"]
frontend_domains = ["http://localhost:8000"]
```

http://localhost:8080

```bash
cd ./parser-api/
cargo watch -x run
```

---

## Frontend

./static/config.js
```js
window.APP_CONFIG = Object.freeze({
    posts_domain: "https://uri.com",
    backend_domain: "https://uri.com",
});
```

http://localhost:8000

```bash
cd ./parser-web/
trunk serve
```

