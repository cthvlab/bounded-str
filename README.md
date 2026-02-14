# BoundedStr: No-Std, Zero-Heap strings with compile-time constraints

BoundedStr are secure string types for Rust that implement the "parse, don't validate" approach. Parsing immediately converts raw data into types with guaranteed correctness (length, format), eliminating repeated checks in business logic.

## Basic concept

Crate creates generalized string types with compile-time parameters: `MIN`, `MAX`, `MAX_BYTES`. The check takes place in `new()` or deserialization — success means full validity. Storing in `[u8; MAX_BYTES]` on the stack provides zero-heap and `#![no_std]` compatibility.

## Key Features

-**Compile-time constraints**: `MIN ≤ MAX ≤ MAX_BYTES` are checked by the compiler.

-**Runtime parsing**: `BoundedStr::new(&str)` validates length/format, returns `Result<Self, BoundedStrError>`.

- **Policies (Traits)**:

* `LengthPolicy`: `Bytes` (by bytes, O(1)) or `Chars` (Unicode characters, O(n)).

-`FormatPolicy`: `AllowAll`, `AsciiOnly` + expandable (e.g. `AlphaNumeric`).

-**Security**: `debug_assert!` in `as_str()`, features `zeroize` (zeroing at `Drop`), `constant-time` (`ct_eq` against timing attacks).

- **Integration**: `Deref<Target=str>`, `Display`, `FromStr`, `TryFrom<&str>`, `serde` with auto-parsing.

## Usage

```rust

use bounded_str::{StackStr, FlexStr, Bytes, Chars, AsciiOnly};
use serde::Deserialize;

// Matrix spec-compliant types — showcase ALL crate features:

// 1. Bytes (O(1)) — Room IDs, technical strings
type RoomId     = StackStr<8, 255, 255, Bytes>; 

// 2. Chars (Unicode) — usernames with Cyrillic/emoji  
type Username   = StackStr<1, 255, 1024, Chars>; 

// 3. Bytes + AsciiOnly — device IDs, technical strings
type DeviceId   = StackStr<1, 32, 32, Bytes, AsciiOnly>;

// 4. Passwords — short, zeroize-enabled
type Password   = StackStr<8, 128, 128, Bytes>;  

// 5. JWT tokens — large buffer (2KiB)
type Token      = FlexStr<16, 2048, 2048, Bytes>; 

// 6. HTML content — large, auto heap (up to 64KiB)
type HtmlBody   = FlexStr<0, 65536, 65536, Bytes>;  

// JSON auto-validation! (parse, don't validate)
#[derive(Deserialize)]
struct LoginRequest {
    username:    Username,       // Unicode OK, 1-255 chars (1KiB buffer)
    device_id:   DeviceId,       // ASCII only, 1-32 bytes (32B buffer)
    password:    Password,       // 8-128 bytes, auto zeroize (128B buffer)
    access_token: Option<Token>, // JWT up to 2KiB buffer
    room_id:     Option<RoomId>, // Matrix room everywhere!
    html_body:   HtmlBody,       // Large HTML → auto heap
}

// Real usage — invalid JSON fails automatically!
let json = r#"{
    "username": "alexey", 
    "device_id": "DEV123",
    "password": "MySecurePass123",
    "html_body": "<p>Matrix <b>rich content</b> up to 64KiB</p>"
}"#;

let req: Result<LoginRequest, _> = serde_json::from_str(json);
// Short password or oversized HTML → serde fails instantly! No manual if-checks needed.

```


## Cargo Features

```toml

[dependencies]

bounded-str = { version = "0.1", features = ["serde", "zeroize", "constant-time"] }

```

-`serde`: Deserialization with parsing.

-`zeroize`: Auto-zeroing the buffer (passwords/tokens).

- `constant-time`: `ct_eq(&self, other)`.

## Restrictions

-`Chars` — O(n) time.

-`MAX_BYTES ≤ 4KiB` (stack).

- No `Copy` (large buffers).