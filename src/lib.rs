#![no_std]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{vec::Vec, string::String};

use core::{
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    str::{self, FromStr},
};

pub trait LengthPolicy {
    fn logical_len(s: &str) -> usize;
    fn const_logical_len(s: &'static str) -> usize;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Bytes;
impl LengthPolicy for Bytes {
    #[inline(always)]
    fn logical_len(s: &str) -> usize { s.len() }
    #[inline(always)]
    fn const_logical_len(s: &'static str) -> usize { s.len() }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Chars;
impl LengthPolicy for Chars {
    #[inline(always)]
    fn logical_len(s: &str) -> usize { s.chars().count() }
    #[inline(always)]
    fn const_logical_len(s: &'static str) -> usize { s.chars().count() }
}

pub trait FormatPolicy {
    fn check_format(s: &str) -> bool;
    fn const_check_format(s: &'static str) -> bool;
}


#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAll;
impl FormatPolicy for AllowAll {
    #[inline(always)]
    fn check_format(_: &str) -> bool { true }
    #[inline(always)]
    fn const_check_format(_: &'static str) -> bool { true }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AsciiOnly;
impl FormatPolicy for AsciiOnly {
    #[inline(always)]
    fn check_format(s: &str) -> bool { s.is_ascii() }
    #[inline(always)]
    fn const_check_format(s: &'static str) -> bool { s.is_ascii() }
}


pub trait StoragePolicy {
    const ALLOW_HEAP: bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StackOnly;
impl StoragePolicy for StackOnly {
    const ALLOW_HEAP: bool = false;  // Только [u8; MAX_BYTES]
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HeapAllowed; 
impl StoragePolicy for HeapAllowed {
    const ALLOW_HEAP: bool = true;   // Vec<u8> если > MAX_BYTES
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedStrError {
    TooShort,
    TooLong,
    TooManyBytes,
    InvalidContent,
    MutationFailed,
}


pub struct BoundedStr<
    const MIN: usize,
    const MAX: usize,
    const MAX_BYTES: usize,
    L: LengthPolicy = Bytes,
    F: FormatPolicy = AllowAll,
    S: StoragePolicy = StackOnly,  
> {
    len: usize,
    buf: [u8; MAX_BYTES],    
    #[cfg(feature = "alloc")]
	heap_buf: Option<Vec<u8>>,  
    _marker: PhantomData<(L, F, S)>,
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F, S>
    BoundedStr<MIN, MAX, MAX_BYTES, L, F, S>
where
    L: LengthPolicy,
    F: FormatPolicy,
    S: StoragePolicy,
{
    const _CHECK_BOUNDS: () = {
        assert!(MIN <= MAX, "MIN must be <= MAX");
        assert!(MAX <= MAX_BYTES, "MAX must be <= MAX_BYTES");
    };

	
	#[inline]
	pub fn new(input: &str) -> Result<Self, BoundedStrError> {
        let byte_len = input.len();
        let logical_len = L::logical_len(input);

        if logical_len < MIN { return Err(BoundedStrError::TooShort); }
        if logical_len > MAX { return Err(BoundedStrError::TooLong); }
        if !F::check_format(input) { return Err(BoundedStrError::InvalidContent); }

if byte_len <= MAX_BYTES || !S::ALLOW_HEAP {
    // stack-only
    let mut buf = [0u8; MAX_BYTES];
    buf[..byte_len].copy_from_slice(input.as_bytes());
    #[cfg(feature = "alloc")]
    let heap_buf = None;
    Ok(Self { len: byte_len, buf, #[cfg(feature = "alloc")] heap_buf, _marker: PhantomData })
} else {
    #[cfg(feature = "alloc")]
    {
        let heap_vec = input.as_bytes().to_vec();
        let mut buf = [0u8; MAX_BYTES];
        buf[..MAX_BYTES].copy_from_slice(&heap_vec[..MAX_BYTES]);
        Ok(Self { len: byte_len, buf, heap_buf: Some(heap_vec), _marker: PhantomData })
    }
}
    }


    #[inline(always)]
pub fn const_new(input: &'static str) -> Result<Self, BoundedStrError> {
    let byte_len = input.len();
    if byte_len > MAX_BYTES { return Err(BoundedStrError::TooManyBytes); }
    let logical_len = L::const_logical_len(input);
    if logical_len < MIN { return Err(BoundedStrError::TooShort); }
    if logical_len > MAX { return Err(BoundedStrError::TooLong); }
    if !F::const_check_format(input) { return Err(BoundedStrError::InvalidContent); }

    let mut buf = [0u8; MAX_BYTES];
    buf[..byte_len].copy_from_slice(input.as_bytes());

    #[cfg(feature = "alloc")]
    let heap_buf = None; // const_new всегда stack-only

    Ok(Self { len: byte_len, buf, #[cfg(feature = "alloc")] heap_buf, _marker: PhantomData })
}

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        if self.len <= MAX_BYTES || !S::ALLOW_HEAP {
            debug_assert!(core::str::from_utf8(&self.buf[..self.len]).is_ok());
            unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
        } else {
            #[cfg(feature = "alloc")]
{
    if let Some(heap_vec) = &self.heap_buf {
        unsafe { core::str::from_utf8_unchecked(heap_vec) }
    } else {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}
        }
    }

    #[inline(always)] pub fn len_bytes(&self) -> usize { self.len }
    #[inline(always)] pub fn len_logical(&self) -> usize { L::logical_len(self.as_str()) }

    pub fn mutate<Mut, Res>(&mut self, mutator: Mut) -> Result<Res, BoundedStrError>
    where Mut: FnOnce(&mut [u8]) -> Res {
        let old_len = self.len;
        let res = mutator(&mut self.buf[..old_len]);
        if let Ok(s) = core::str::from_utf8(&self.buf[..old_len]) {
            if s.len() != old_len {
                return Err(BoundedStrError::MutationFailed);
            }
            let logical_len = L::logical_len(s);
            if logical_len < MIN || logical_len > MAX || !F::check_format(s) {
                return Err(BoundedStrError::MutationFailed);
            }
            Ok(res)
        } else {
            Err(BoundedStrError::MutationFailed)
        }
    }

    #[cfg(feature = "constant-time")]
    #[inline]
    pub fn ct_eq(&self, other: &Self) -> bool {
        if self.len != other.len { return false; }
        let mut diff: u8 = 0;
        for i in 0..MAX_BYTES {
            let a = if i < self.len { self.buf[i] } else { 0 };
            let b = if i < other.len { other.buf[i] } else { 0 };
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> PartialEq
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    fn eq(&self, other: &Self) -> bool { self.as_str() == other.as_str() }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> Eq
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy {}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> PartialEq<&str>
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    fn eq(&self, other: &&str) -> bool { self.as_str() == *other }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> Deref
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    type Target = str;
    fn deref(&self) -> &str { self.as_str() }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> TryFrom<&str>
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    type Error = BoundedStrError;
    fn try_from(value: &str) -> Result<Self, Self::Error> { Self::new(value) }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> FromStr
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    type Err = BoundedStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::new(s) }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> Hash
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_str().hash(state); }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F> Display
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
where L: LengthPolicy, F: FormatPolicy
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F, S> fmt::Debug
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F, S>
where
    L: LengthPolicy,
    F: FormatPolicy,
    S: StoragePolicy,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundedStr")
            .field("value", &self.as_str())
            .field("len_bytes", &self.len_bytes())
            .field("len_logical", &self.len_logical())
            .finish()
    }
}

#[cfg(feature = "zeroize")]
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L, F, S> Drop
    for BoundedStr<MIN, MAX, MAX_BYTES, L, F, S>
where
    L: LengthPolicy,
    F: FormatPolicy,
    S: StoragePolicy,
{
    fn drop(&mut self) {
        for b in &mut self.buf { *b = 0; }

        #[cfg(feature = "alloc")]
		if S::ALLOW_HEAP && let Some(heap_vec) = &mut self.heap_buf {
			for b in heap_vec.iter_mut() {
				*b = 0;
			}
		}
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    pub struct Visitor<const MIN: usize, const MAX: usize, const MAXB: usize, L, F> {
        _marker: PhantomData<(L, F)>,
    }

    impl<'de, const MIN: usize, const MAX: usize, const MAXB: usize, L, F> de::Visitor<'de>
        for Visitor<MIN, MAX, MAXB, L, F>
    where
        L: LengthPolicy + 'static,
        F: FormatPolicy + 'static,
    {
        type Value = BoundedStr<MIN, MAX, MAXB, L, F>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string [{MIN}..={MAX}]")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: de::Error,
        {
            BoundedStr::new(v).map_err(|err| match err {
                BoundedStrError::TooShort | BoundedStrError::TooLong | BoundedStrError::TooManyBytes =>
                    de::Error::invalid_length(v.len(), &self),
                BoundedStrError::InvalidContent =>
                    de::Error::invalid_value(de::Unexpected::Str(v), &self),
                _ => de::Error::custom("unexpected error"),
            })
        }

        #[cfg(feature = "alloc")]
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: de::Error,
        {
            self.visit_str(&v)
        }
    }

    impl<'de, const MIN: usize, const MAX: usize, const MAXB: usize, L, F> Deserialize<'de>
        for BoundedStr<MIN, MAX, MAXB, L, F>
    where
        L: LengthPolicy + 'static,
        F: FormatPolicy + 'static,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
        {
            deserializer.deserialize_str(Visitor::<MIN, MAX, MAXB, L, F> { _marker: PhantomData })
        }
    }

    impl<const MIN: usize, const MAX: usize, const MAXB: usize, L, F> Serialize
        for BoundedStr<MIN, MAX, MAXB, L, F>
    where
        L: LengthPolicy,
        F: FormatPolicy,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
            serializer.serialize_str(self.as_str())
        }
    }
}



/// Stack-only (<4KiB)
pub type StackStr<const MIN: usize, const MAX: usize, const MAXB: usize = MAX, L = Bytes, F = AllowAll> = 
    BoundedStr<MIN, MAX, MAXB, L, F, StackOnly>;

/// Stack + heap fallback (HTML 64KiB)
pub type FlexStr<const MIN: usize, const MAX: usize, const MAXB: usize = 4096, L = Bytes, F = AllowAll> = 
    BoundedStr<MIN, MAX, MAXB, L, F, HeapAllowed>;
