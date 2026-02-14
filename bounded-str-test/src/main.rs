use bounded_str::BoundedStr;
use std::time::Instant;
use std::io::{self, Write};
use serde::Deserialize;

// Username: длина 3..16, только символы ASCII
type Username = BoundedStr<3, 16, 16, bounded_str::Chars, bounded_str::AsciiOnly>;

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenPolicy;

// TokenPolicy: длина до 128, только ASCII алфавитно-цифровые символы
impl bounded_str::FormatPolicy for TokenPolicy {
    fn check_format(s: &str) -> bool {
        s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric())
    }
    fn const_check_format(s: &'static str) -> bool {
        s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

// Примеры вариантов Token:
// type Token = BoundedStr<1, 128, 128>; - пропустит emoji, не строго
// type Token = BoundedStr<1, 128, 128, bounded_str::Bytes, bounded_str::AsciiOnly>; - без точной проверки
type Token = bounded_str::BoundedStr<1, 128, 128, bounded_str::Chars, TokenPolicy>;

// Структура для десериализации JSON
#[derive(Deserialize)]
struct InputData {
    username: String,
    token: String,
}

fn main() {
    println!("Interactive BoundedStr Tester");
    println!("Enter JSON like {{\"username\":\"Alice\",\"token\":\"a1b2c3d4e5\"}}");
    println!("Or enter space-separated: username token");
    println!("Type 'exit' to quit.\n");

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();
        if input.is_empty() { continue; }
        if input.eq_ignore_ascii_case("exit") { break; }

        // Засекаем общее время цикла
        let start = Instant::now();

        // Засекаем чистый парсинг + BoundedStr
        let parse_start = Instant::now();

        let (username_str, token_str) = if input.starts_with('{') {
            // Попытка JSON
            match serde_json::from_str::<InputData>(input) {
                Ok(data) => (data.username, data.token),
                Err(e) => {
                    eprintln!("Failed to parse JSON: {}", e);
                    continue;
                }
            }
        } else {
            // Простая пробел-разделённая строка
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() != 2 {
                eprintln!("Expected two values: username token");
                continue;
            }
            (parts[0].to_string(), parts[1].to_string())
        };

        // Проверка Username
        let user = match Username::new(&username_str) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Username error: {:?}", e);
                continue;
            }
        };

        // Проверка Token
        let token = match Token::new(&token_str) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Token error: {:?}", e);
                continue;
            }
        };

        let parse_elapsed = parse_start.elapsed(); // время только на парсинг и валидацию


        // Рендер в консоль 
        println!(
            "Username: {}, bytes: {}, chars: {}",
            user,
            user.len_bytes(),
            user.len_logical()
        );
        println!(
            "Token: {}, bytes: {}, chars: {}",
            token,
            token.len_bytes(),
            token.len_logical()
        );

        let elapsed = start.elapsed(); // общее время цикла включая вывод
        println!("Parse + validation time: {:.6} seconds", parse_elapsed.as_secs_f64());
        println!("Total cycle time (including console render): {:.6} seconds\n", elapsed.as_secs_f64());
    }

    println!("Exiting interactive tester.");
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
	use bounded_str::BoundedStrError;
    type Username = BoundedStr<3, 16, 32, bounded_str::Chars, bounded_str::AsciiOnly>;
    type Token = BoundedStr<1, 128, 128, bounded_str::Chars, TokenPolicy>;

    #[test]
    fn test_valid_username_and_token() {
        let username = "Alice123";
        let token = "a1b2c3d4e5";

        let u = Username::new(username).expect("Valid username should pass");
        let t = Token::new(token).expect("Valid token should pass");

        assert_eq!(u.as_str(), username);
        assert_eq!(t.as_str(), token);
    }

    #[test]
    fn test_username_too_short() {
        let err = Username::new("Al").unwrap_err();
        assert!(matches!(err, BoundedStrError::TooShort));
    }

    #[test]
    fn test_username_too_long() {
        let long = "A".repeat(17);
        let err = Username::new(&long).unwrap_err();
        assert!(matches!(err, BoundedStrError::TooLong));
    }

    #[test]
    fn test_username_invalid_chars() {
        let err = Username::new("Bob🔥").unwrap_err();
        assert!(matches!(err, BoundedStrError::InvalidContent));
    }

    #[test]
    fn test_token_too_long() {
        let long_token = "a".repeat(129);
        let err = Token::new(&long_token).unwrap_err();
        assert!(matches!(err, BoundedStrError::TooLong | BoundedStrError::TooManyBytes));
    }

    #[test]
    fn test_token_invalid_chars() {
        let err = Token::new("🔥🔥🔥").unwrap_err();
        assert!(matches!(err, BoundedStrError::InvalidContent));
    }

    #[test]
    fn test_json_parsing_valid() {
        let json_data = json!({
            "username": "Alice",
            "token": "abc123XYZ"
        })
        .to_string();

        let parsed: InputData = serde_json::from_str(&json_data).expect("JSON should parse");
        let _u = Username::new(&parsed.username).expect("Username valid");
        let _t = Token::new(&parsed.token).expect("Token valid");
    }

    #[test]
    fn test_json_parsing_invalid_username() {
        let json_data = json!({
            "username": "Al",
            "token": "abc123"
        })
        .to_string();

        let parsed: InputData = serde_json::from_str(&json_data).expect("JSON should parse");
        let err = Username::new(&parsed.username).unwrap_err();
        assert!(matches!(err, BoundedStrError::TooShort));
    }

    #[test]
    fn test_json_parsing_invalid_token() {
        let json_data = json!({
            "username": "Alice",
            "token": "🔥🔥🔥"
        })
        .to_string();

        let parsed: InputData = serde_json::from_str(&json_data).expect("JSON should parse");
        let err = Token::new(&parsed.token).unwrap_err();
        assert!(matches!(err, BoundedStrError::InvalidContent));
    }
}


#[cfg(test)]
mod heap_tests {
    use super::*;
    use bounded_str::{FlexStr, BoundedStrError};
    
    // HTML-like big content: stack+heap fallback
    type HtmlBody = FlexStr<0, 65536, 4096, bounded_str::Bytes>; // MAXB < MAX → авто-хип
    type BigToken = FlexStr<1, 2048, 128, bounded_str::Bytes, TokenPolicy>;

    #[test]
    fn heap_small_stays_stack() {
        let s = "short content";
        let b = HtmlBody::new(s).expect("Small string should fit on stack");
        assert_eq!(b.as_str(), s);
        assert!(b.len_bytes() <= 4096);
    }

    #[test]
    fn heap_large_fallback_to_heap() {
        let large = "A".repeat(5000); // больше MAXB=4096 → heap
        let b = HtmlBody::new(&large).expect("Large string should allocate heap");
        assert_eq!(b.len_bytes(), 5000);
        assert_eq!(b.as_str(), large);
    }

    #[test]
    fn heap_max_allowed_length() {
        let max = "B".repeat(65536);
        let b = HtmlBody::new(&max).expect("Max allowed string");
        assert_eq!(b.len_bytes(), 65536);
        assert_eq!(b.as_str(), max);
    }

    #[test]
    fn heap_too_long_error() {
        let too_long = "C".repeat(65537);
        let err = HtmlBody::new(&too_long).unwrap_err();
        assert!(matches!(err, BoundedStrError::TooLong | BoundedStrError::TooManyBytes));
    }

    #[test]
    fn heap_token_validation() {
        let valid_token = "abc123XYZ";
        let invalid_token = "🔥🔥🔥";

        let t = BigToken::new(valid_token).expect("Valid token passes");
        assert_eq!(t.as_str(), valid_token);

        let err = BigToken::new(invalid_token).unwrap_err();
        assert!(matches!(err, BoundedStrError::InvalidContent));
    }

    #[test]
    fn mutate_heap_string() {
        let mut b = HtmlBody::new("Hello world").unwrap();
        let res = b.mutate(|buf| {
            buf[0] = b'J';
            42
        }).unwrap();
        assert_eq!(res, 42);
        assert_eq!(b.as_str(), "Jello world");
    }
}