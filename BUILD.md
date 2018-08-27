# Build instructions

## Dependencies
- `Rust nightly` (strongly recommended to use [rustup](https://rustup.rs/))
- `OpenSSL` (should be available in path, install through your system's package manager or use the [prebuilt binaries](https://wiki.openssl.org/index.php/Binaries))
- `NodeJS` (required to build the web-vault, (install through your system's package manager or use the [prebuilt binaries](https://nodejs.org/en/download/))


## Run/Compile
```sh
# Compile and run
cargo run
# or just compile (binary located in target/release/bitwarden_rs)
cargo build --release
```

When run, the server is accessible in [http://localhost:80](http://localhost:80).

### Install the web-vault
Clone the git repository at [bitwarden/web](https://github.com/bitwarden/web) and checkout the latest release tag (e.g. v2.1.1):
```sh
# clone the repository
git clone https://github.com/bitwarden/web.git web-vault
cd web-vault
# switch to the latest tag
git checkout "$(git tag | tail -n1)"
```

Apply the patch file from `docker/set-vault-baseurl.patch`:
```sh
# In the Vault repository directory
git apply /path/to/bitwarden_rs/docker/set-vault-baseurl.patch
```

Then, build the Vault:
```sh
npm run sub:init
npm install
npm run dist
```

Finally copy the contents of the `build` folder into the `bitwarden_rs/web-vault` folder.

# Configuration
The available configuration options are documented in the default `.env` file, and they can be modified by uncommenting the desired options in that file or by setting their respective environment variables. Look at the README file for the main configuration options available.

Note: the environment variables override the values set in the `.env` file.

## How to recreate database schemas (for developers)
Install diesel-cli with cargo:
```sh
cargo install diesel_cli --no-default-features --features sqlite-bundled
```

Make sure that the correct path to the database is in the `.env` file.

If you want to modify the schemas, create a new migration with:
```
diesel migration generate <name>
```

Modify the *.sql files, making sure that any changes are reverted in the down.sql file.

Apply the migrations and save the generated schemas as follows:
```sh
diesel migration redo

# This step should be done automatically when using diesel-cli > 1.3.0
# diesel print-schema > src/db/schema.rs
```
