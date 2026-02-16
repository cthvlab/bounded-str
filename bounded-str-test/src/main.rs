use bounded_str::BoundedStr;
use std::time::Instant;
use std::io::{self, Write};
use serde::Deserialize;

// Username: длина 3..16, только символы ASCII
type Username = BoundedStr<3, 16, 16, bounded_str::Chars, bounded_str::AsciiOnly>;


/* Важное архитектурное замечание
Хотя `FormatPolicy` позволяет проверять формат данных (например, Email или Regex), помните о философии "Parse, don't validate":
- Сложность: Попытка идеально валидировать Email через политики может привести к созданию «хрупкого» кода.  
- Рекомендация: Используйте политики для структурных ограничений (длина, кодировка ASCII, отсутствие управляющих символов).  
- Бизнес-логика: Глубокую проверку (существует ли домен, соответствует ли Email RFC 5322) лучше выносить в специализированные парсеры, которые преобразуют BoundedStr в еще более строгие типы данных.
*/
#[derive(Clone, Copy, Debug, Default)]
pub struct TokenPolicy;

impl bounded_str::FormatPolicy for TokenPolicy {
    #[inline(always)]
    fn check(s: &str) -> bool {
        s.len() <= 128 && s.chars().all(|c| c.is_ascii_alphanumeric())
    }
}
// Не рекомендуется
type Token = bounded_str::BoundedStr<1, 128, 128, bounded_str::Chars, TokenPolicy>; 

// Примеры вариантов Token:
// type Token = BoundedStr<1, 128, 128>; - пропустит emoji, не строго
// type Token = BoundedStr<1, 128, 128, bounded_str::Bytes, bounded_str::AsciiOnly>; - без точной проверки


// Структура для десериализации JSON
#[derive(Deserialize)]
struct InputData {
    username: String,
    token: String,
}

fn main() {
let ascii = r#"
   ___                       _          _ __ _        
  / __\ ___  _   _ _ __   __| | ___  __| / _\ |_ _ __ 
 /__\/// _ \| | | | '_ \ / _` |/ _ \/ _` \ \| __| '__|
/ \/  \ (_) | |_| | | | | (_| |  __/ (_| |\ \ |_| |   
\_____/\___/ \__,_|_| |_|\__,_|\___|\__,_\__/\__|_|  
"#; println!("\x1b[32m{}\x1b[0m", ascii);
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


//////////////////////////////////////
// cargo test --release -- --nocapture
//////////////////////////////////////

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
        let res = b.mutate(|buf, _len| { 
            buf[0] = b'J';
            42
        }).unwrap();
        assert_eq!(res, 42);
        assert_eq!(b.as_str(), "Jello world");
    }
}

#[cfg(test)]
mod stress_tests {
    use bounded_str::FlexStr;
    use std::time::Instant;

    // Heap-heavy FlexStr: small stack buffer, всё >8KiB идёт на heap
    type BigChunk = FlexStr<1, {1024*1024}, {8*1024}, bounded_str::Bytes>;

    #[test]
    fn stress_1gb() {
        let chunk_size = 1024 * 1024; // 1 MiB per chunk
        let total_chunks = 1024;      // 1024 * 1 MiB = 1 GiB total
        let data_chunk = "A".repeat(chunk_size);
        let total_bytes = chunk_size * total_chunks;

        println!("Starting creation of 1 GiB in {} chunks...", total_chunks);
        let start_create = Instant::now();

        let mut big_vec = Vec::with_capacity(total_chunks);

        // Create all chunks
        for _ in 0..total_chunks {
            let chunk = BigChunk::new(&data_chunk).expect("Chunk should fit");
            big_vec.push(chunk);
        }

        let duration_create = start_create.elapsed();
        let mbps_create = total_bytes as f64 / (1024.0 * 1024.0) / duration_create.as_secs_f64();
        println!(
            "Created 1 GiB in {:.6} sec (~{:.2} MB/s)",
            duration_create.as_secs_f64(),
            mbps_create
        );

        // Mutate all chunks (flip first byte)
        println!("Starting mutation of all chunks...");
        let start_mut = Instant::now();

        for chunk in &mut big_vec {
            let _ = chunk.mutate(|buf, _len| {
                buf[0] = if buf[0] == b'A' { b'B' } else { b'A' };
            });
        }

        let duration_mut = start_mut.elapsed();
        let mbps_mut = total_bytes as f64 / (1024.0 * 1024.0) / duration_mut.as_secs_f64();
        println!(
            "Mutated 1 GiB in {:.6} sec (~{:.2} MB/s)",
            duration_mut.as_secs_f64(),
            mbps_mut
        );

        // Read all chunks (sum lengths)
        println!("Starting read of all chunks...");
        let start_read = Instant::now();

        let total_len: usize = big_vec.iter().map(|s| s.len_bytes()).sum();

        let duration_read = start_read.elapsed();
        let mbps_read = total_bytes as f64 / (1024.0 * 1024.0) / duration_read.as_secs_f64();
        println!(
            "Read 1 GiB in {:.6} sec (~{:.2} MB/s), total bytes: {}",
            duration_read.as_secs_f64(),
            mbps_read,
            total_len
        );

        assert_eq!(total_len, total_bytes);
    }
}


#[cfg(test)]
mod security_tests {
    use super::*;
	use bounded_str::*;

    // 1. Тест на Constant-Time сравнение
    // Мы проверяем, что флаг корректно работает для разных типов
    #[test]
    #[cfg(feature = "constant-time")]
    fn test_constant_time_equality() {
        type Secret = BoundedStr<1, 32, 32, Bytes, AllowAll, true>;
        
        let s1 = Secret::new("password123").unwrap();
        let s2 = Secret::new("password123").unwrap();
        let s3 = Secret::new("wrongpassword").unwrap();

        // Проверяем, что логика сравнения выдает верный результат
        assert!(s1 == s2); 
        assert!(s1 != s3);
        
        // Тестируем сравнение с разными длинами (оно должно возвращать false)
        let s4 = Secret::new("pass").unwrap();
        assert!(s1 != s4);
    }

    // 2. Тест на Zeroize (Логический)
    // Напрямую проверить RAM после drop сложно без внешних инструментов, 
    // но мы можем проверить, что мутатор корректно затирает временную копию.
    #[test]
    #[cfg(feature = "zeroize")]
    fn test_zeroize_mutation_failure_cleanup() {
        type Secret = BoundedStr<5, 10, 32, Bytes, AllowAll, true>;
        let mut s = Secret::new("valid").unwrap();

        // Пытаемся сделать невалидную мутацию (слишком короткая строка)
        // Внутри сработает clear_temp_vec
        let res = s.mutate(|buf, len| {
            buf[0] = b'X';
            *len = 1; // Ошибка: меньше MIN (5)
        });

        assert!(res.is_err());
        // Оригинал не должен измениться
        assert_eq!(s.as_str(), "valid");
    }

    // 3. Тест на совместимость алиасов
    #[test]
    fn test_alias_zeroize_defaults() {
        // Проверяем, что StackStr по умолчанию Z = false (компилируется)
        let _s: StackStr<1, 10> = StackStr::new("test").unwrap();
        
        // Проверяем ручную установку Z в алиасе
        type MySecret = StackStr<1, 10, 10, Bytes, AllowAll, true>;
        let _sec = MySecret::new("secret").unwrap();
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_flex_str_zeroize() {
        // Проверяем FlexStr с кучей и флагом затирания
        type SecretHeap = FlexStr<1, 100, 10, Bytes, AllowAll, true>;
        let mut s = SecretHeap::new("long_secret_string").unwrap(); // Уйдет в кучу
        
        assert!(s.len_bytes() > 10); 
        
        // Мутация в куче
        s.mutate(|buf, _len| {
            buf[0] = b'Z';
        }).unwrap();
        
        assert_eq!(&s.as_str()[0..1], "Z");
    }
	
	#[test]
	fn crash_test_emoji_butcher() {
		type UnicodeStr = StackStr<1, 10, 20, Chars>;
		let mut s = UnicodeStr::new("🔥").unwrap();
		
		// Пытаемся испортить второй байт эмодзи вручную
		let res = s.mutate(|buf, _len| {
			if buf.len() > 1 {
				buf[1] = 0xFF; // Делаем байт невалидным для UTF-8
			}
		});

		// Должно вернуть MutationFailed, а старая "огонь" должна выжить
		assert!(res.is_err());
		assert_eq!(s.as_str(), "🔥");
	}
	#[test]
	fn crash_test_stack_boundary() {
		// Буфер ровно 5 байт
		type Fixed = StackStr<1, 5, 5, Bytes>;
		let mut s = Fixed::new("1234").unwrap();

		// Пытаемся записать 6 байт через мутатор
		let res = s.mutate(|buf, len| {
			// Мы физически имеем доступ к 5 байтам массива
			for i in 0..5 { buf[i] = b'A'; }
			*len = 6; // Лжём про длину
		});

		// Должно поймать TooManyBytes
		assert!(res.is_err());
		assert_eq!(s.as_str(), "1234");
	}

	#[test]
	fn crash_test_zero_buffer() {
		// MAX_BYTES = 0, значит СРАЗУ в кучу
		#[cfg(feature = "alloc")]
		{
			type HeapOnly = FlexStr<1, 100, 0, Bytes>;
			let s = HeapOnly::new("A").expect("Should go to heap instantly");
			assert_eq!(s.as_str(), "A");
			
			// Проверяем, что мутатор не падает при работе с пустым стеком
			let mut s = s;
			s.mutate(|buf, _len| {
				buf[0] = b'B';
			}).unwrap();
			assert_eq!(s.as_str(), "B");
		}
	}
	
	#[test]
	#[cfg(feature = "alloc")]
	fn crash_test_heap_overflow() {
		// Лимит 100 байт, в стеке 10
		type LimitedFlex = FlexStr<1, 100, 10, Bytes>;
		let mut s = LimitedFlex::new("12345678901").unwrap(); // Уже в куче (11 байт)

		let res = s.mutate(|_buf, len| {
			// Мутатор видит срез длиной MAX (100)
			// Пытаемся сказать, что записали 200 байт
			*len = 200;
		});

		assert!(matches!(res, Err(BoundedStrError::TooManyBytes)));
	}
	
	#[test]
	fn crash_test_panic_safety() {
		type Secret = StackStr<1, 10, 10, Bytes, AllowAll, true>;
		let mut s = Secret::new("secret").unwrap();

		// Запускаем мутатор, который паникует
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let _ = s.mutate(|_buf, _len| {
				panic!("Boom!");
			});
		}));

		assert!(result.is_err());
		// Проверяем, что после паники объект s всё еще валиден и содержит старые данные
		assert_eq!(s.as_str(), "secret");
	}



}


