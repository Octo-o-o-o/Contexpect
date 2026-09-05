# Boundary against Python's `json`

*Generated from `BOUNDARY` in `src/boundary.rs` by `cargo run -p ctxpect-schema --example render_boundary`. Editing this file directly fails `cargo test`.*

## Accepted by both

- `{"v":[1,{"k":"值"},null,true,-7]}` — accepted by both — Objects, arrays, strings, `i64` integers, booleans and null. Within this subset the canonical form and its digest match the acceptance generator.

## Rejected here, accepted by Python

- `{"v":1000000000000000.25}` — rejected here, accepted by Python — Python's shortest representation breaks rounding ties to even, Rust's breaks away from zero, so the same float can render differently and digest differently.
- `{"v":1.0}` — rejected here, accepted by Python — A float literal; floats are not supported.
- `{"v":1e16}` — rejected here, accepted by Python — A float literal; floats are not supported.
- `{"v":9223372036854775808}` — rejected here, accepted by Python — Outside `i64`. Python has arbitrary precision integers; this crate does not.
- `{"v":NaN}` — rejected here, accepted by Python — Not defined by JSON. Python accepts it as an extension; this crate does not.
- `{"v":Infinity}` — rejected here, accepted by Python — Not defined by JSON. Python accepts it as an extension; this crate does not.
- `{"v":"\ud800"}` — rejected here, accepted by Python — A lone surrogate. Python yields a `str` holding it; Rust's `String` cannot.

## Rejected by both

- `{"v":01}` — rejected by both — Malformed under the JSON grammar.
- `{"v":"\u+041"}` — rejected by both — Malformed under the JSON grammar.
- `{"v":1.}` — rejected by both — Malformed under the JSON grammar.
- `{"v":0.e5}` — rejected by both — Malformed under the JSON grammar.
- `{"v":"ab"}` — rejected by both — Malformed under the JSON grammar.
- `{"v":1,}` — rejected by both — Malformed under the JSON grammar.
- `{'v':1}` — rejected by both — Malformed under the JSON grammar.
