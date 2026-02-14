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

## Examples of types

```rust

use bounded_str::{BoundedStr, Bytes, Chars, AsciiOnly};

// Matrix Room ID: 8-128 bytes

type MatrixRoomId = BoundedStr<8, 128, 128, Bytes>;

// Username: 3-32 characters, ASCII

type Username = BoundedStr<3, 32, 64, Chars, AsciiOnly>;

// Token: 16-64 bytes

type SessionToken = BoundedStr<16, 64, 64, Bytes>;

```

## Usage

```rust

let room = MatrixRoomId::new("!abc123:matrix.org").unwrap();

assert_eq!(&*room, "!abc123:matrix.org "); // Deref as &str

assert_eq!(room.len_bytes(), 19);

// Parse, don't validate in serde

#[derive(serde::Deserialize)]

struct LoginResponse {

access_token: sessionToken, // Auto-length check during deserialization

}

```

## Comparison of approaches

| Aspect | BoundedStr (Parse) | The usual String + validate() |

|--------|--------------------|-----------------------------|

| Creation | `Type::new(str)?` → guaranteed type | `if validate(str) { String::from(str) }` |

| Access | `&*bound.as_str()` without checks | Repeated `if len_ok(s)` everywhere |

| Memory | Stack `[u8; N]`, zero-heap | Heap, allocation |

| Matrix | Ideal for roomId/UserId|Runtime DoS from long lines |

| Security | zeroize, ct_eq | Manual cleaning |

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