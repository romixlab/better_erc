//! The document library over mx3's HTTP API (VIEW-26): `by_mpn` ([`super::mpndocs`]) and the docs catalog
//! ([`super::docindex`]) through mx3_api_lab's blocking client, and this computer's library folders
//! ([`mx3_api_lab::library::roots::Roots`]) to resolve a catalog path (`share/path`) into a file here.
//! `library_fetch` (a setting) stays a shell command (`mx3 lib fetch --mpn`): a fetch writes files and runs a
//! browser, which this module has no part in.

use better_erc::parts::mx3::{Connection, Mx3Error};
use mx3_api_lab::blocking::BlockingClient;
use mx3_api_lab::library::roots::Roots;
use std::path::PathBuf;

/// The library's folders when [`ROOTS_ENV`] says nothing: `mx3 lib`'s own default
const DEFAULT_ROOTS: &str = "~/sync/design:~/sync/datasheets:~/sync/books";
/// Lists this computer's library folders, `:`-separated, `~/` for home
const ROOTS_ENV: &str = "MX3_LIB_ROOTS";

/// Why the mx3 connection can't be used, without building a client; `None` when a token is found. Cheap enough
/// for a check made every frame ([`super::FootprintReview::mpn_lookup_blocker`])
pub(super) fn blocker() -> Option<String> {
    (!Connection::from_env().has_token()).then(|| Mx3Error::NoToken.to_string())
}

/// A client of the mx3 backend [`Connection::from_env`] names; refused without a token, since `by_mpn` and the
/// docs catalog both need one
pub(super) fn connect() -> Result<BlockingClient, String> {
    let conn = Connection::from_env();
    let token = conn
        .token()
        .ok_or_else(|| Mx3Error::NoToken.to_string())?
        .to_owned();
    Ok(BlockingClient::new(conn.url).with_token(Some(token)))
}

/// This computer's library folders: [`ROOTS_ENV`] (`:`-separated, `~/` for home), else [`DEFAULT_ROOTS`]
pub(super) fn roots() -> Roots {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let list = std::env::var(ROOTS_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty());
    Roots::parse(list.as_deref().unwrap_or(DEFAULT_ROOTS), home.as_deref())
}
