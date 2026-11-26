# erc_fake_mx3

Test-only crate: a fake mx3 backend for the tests of the mx3 part source (`erc_parts` with feature `mx3`, `berc` with feature `mx3`).

- `http::Server`: a local HTTP/1.1 server on `127.0.0.1` that answers each request from a handler and keeps every request it was sent (method, path, `Authorization` header, body). Standard library only, one connection at a time.
- `FakeMx3`: that server answering `/params/by_mpn`, `/params/sources`, `/params/set` (datasheet URLs only, `datasheets`), `/fp/review/check` and `/fp/review/accept` from a small fixed part catalogue and the reviews accepted so far. Requests and answers are mx3_api_lab's own types, so the fake follows the contract crate; a request without the test token is answered 401.
