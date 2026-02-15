# BoundedStr: Type-Safe Strings with Static Limits and Hybrid Storage

BoundedStr is a library for creating string types with guaranteed invariants.  
It follows the **"Parse, don't validate"** principle: once an object is created, you can trust its length, format, and encoding.

- **Hybrid Storage**: Automatic selection between stack (`StackStr`) and dynamic memory (`FlexStr`) when the byte limit is exceeded.  
- **Policy-Based Design**: You choose how to measure length (bytes or characters) and how to validate content (ASCII, AlphaNumeric, etc.) via traits.  
- **No-Std First**: Full support for embedded systems, `alloc` is required only for `FlexStr`.  

## Core Concepts

### 1. Policies

Instead of hard-coded logic, BoundedStr uses:

- **LengthPolicy**: `Bytes` (fast, O(1)) or `Chars` (Unicode-correct, O(n)).  
- **FormatPolicy**: `AllowAll`, `AsciiOnly`, or your own rules (e.g., `EmailValidator`).  

### 2. Storage Types

- **StackStr**: Always on the stack. If the string does not fit in `MAX_BYTES` — error.  
- **FlexStr**: Tries to fit on the stack, but if `alloc` is enabled and the data is large — transparently moves to the heap.  

## Key Features

- **Compile-time checks**: Checks `MIN <= MAX` at compile time via const assertions.  
- **Transactional Mutation**: `mutate()` allows modifying the string via `&mut [u8]`, rolling back if the result violates type rules.  
- **Zero-cost Deref**: Implements `Deref<Target=str>`, works like a regular string with no overhead.  
- **Security**: Supports `zeroize` for automatic memory clearing (passwords, keys) and constant-time comparison.  

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
````

## Cargo Features

```toml
[dependencies]
bounded-str = { version = "0.1", features = ["serde", "alloc", "zeroize", "constant-time"] }
```

* **serde**: Automatic validation during deserialization.
* **alloc**: Enables `FlexStr` and dynamic memory support.
* **zeroize**: Clears the buffer when it goes out of scope (`Drop`).
* **constant-time**: Protection against timing attacks during string comparisons.

## Limitations

* By default, the stack limit is set to a reasonable size (recommended up to 4KiB).
* The `Chars` policy requires a full scan of the string during creation and mutation.

## Important Architectural Note

Although `FormatPolicy` allows checking data format (e.g., Email or Regex), remember the **"Parse, don't validate"** philosophy:

* **Complexity**: Attempting perfect Email validation via policies may produce fragile code.
* **Recommendation**: Use policies for structural constraints (length, ASCII encoding, absence of control characters).
* **Business Logic**: Deep validation (e.g., domain existence, RFC 5322 compliance) is better handled by specialized parsers that convert BoundedStr into stricter data types.



