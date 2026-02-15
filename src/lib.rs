#![no_std]
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::{vec::Vec};

use core::{
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    str::{self, FromStr},
};

pub trait LengthPolicy {
    fn logical_len(s: &str) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Bytes;
impl LengthPolicy for Bytes {
    #[inline(always)] fn logical_len(s: &str) -> usize { s.len() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Chars;
impl LengthPolicy for Chars {
    #[inline(always)] fn logical_len(s: &str) -> usize { s.chars().count() }
}

pub trait FormatPolicy {
    fn check(s: &str) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AllowAll;
impl FormatPolicy for AllowAll {
    #[inline(always)] fn check(_: &str) -> bool { true }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AsciiOnly;
impl FormatPolicy for AsciiOnly {
    #[inline(always)] fn check(s: &str) -> bool { s.is_ascii() }
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
> {
    len: usize,
    buf: [u8; MAX_BYTES],
    #[cfg(feature = "alloc")]
    heap_buf: Option<Vec<u8>>,
    _marker: PhantomData<(L, F)>,
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    const _CHECK: () = {
        assert!(MIN <= MAX, "MIN must be <= MAX");
        assert!(MAX <= MAX_BYTES, "MAX must be <= MAX_BYTES");
    };


    #[inline(always)]
    pub fn len_bytes(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn len_logical(&self) -> usize {
        L::logical_len(self.as_str())
    }

    pub fn new(s: &str) -> Result<Self, BoundedStrError> {
        let byte_len = s.len();
        let logical_len = L::logical_len(s);

        if logical_len < MIN { return Err(BoundedStrError::TooShort); }
        if logical_len > MAX { return Err(BoundedStrError::TooLong); }
        if !F::check(s) { return Err(BoundedStrError::InvalidContent); }

        #[cfg(feature = "alloc")]
        if byte_len > MAX_BYTES {
            return Ok(Self { 
                len: byte_len, 
                buf: [0u8; MAX_BYTES], 
                heap_buf: Some(s.as_bytes().to_vec()), 
                _marker: PhantomData 
            });
        }

        if byte_len > MAX_BYTES {
            return Err(BoundedStrError::TooManyBytes);
        }

        let mut buf = [0u8; MAX_BYTES];
        buf[..byte_len].copy_from_slice(s.as_bytes());
        
        Ok(Self { 
            len: byte_len, 
            buf, 
            #[cfg(feature = "alloc")] 
            heap_buf: None, 
            _marker: PhantomData 
        })
    }

    pub fn mutate<Mut, R>(&mut self, mutator: Mut) -> Result<R, BoundedStrError>
    where
        Mut: FnOnce(&mut [u8]) -> R
    {
        #[cfg(feature = "alloc")]
        if let Some(ref mut v) = self.heap_buf {
            let mut temp = v.clone();
            let res = mutator(&mut temp);
            if let Ok(s) = core::str::from_utf8(&temp) {
                let l_len = L::logical_len(s);
                if l_len >= MIN && l_len <= MAX && F::check(s) {
                    self.len = temp.len();
                    *v = temp;
                    return Ok(res);
                }
            }
            return Err(BoundedStrError::MutationFailed);
        }

        let mut temp_buf = self.buf; 
        let res = mutator(&mut temp_buf[..self.len]);
        
        if let Ok(s) = core::str::from_utf8(&temp_buf[..self.len]) {
            let l_len = L::logical_len(s);
            if l_len >= MIN && l_len <= MAX && F::check(s) {
                self.buf = temp_buf;
                return Ok(res);
            }
        }
        
        Err(BoundedStrError::MutationFailed)
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        #[cfg(feature = "alloc")]
        if let Some(ref v) = self.heap_buf {
            return unsafe { str::from_utf8_unchecked(v) };
        }
        unsafe { str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}


impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    PartialEq for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    fn eq(&self, other: &Self) -> bool { self.as_str() == other.as_str() }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    Eq for BoundedStr<MIN, MAX, MAX_BYTES, L, F> {}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    PartialEq<&str> for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    fn eq(&self, other: &&str) -> bool { self.as_str() == *other }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    Deref for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    type Target = str;
    fn deref(&self) -> &str { self.as_str() }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    TryFrom<&str> for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    type Error = BoundedStrError;
    fn try_from(s: &str) -> Result<Self, Self::Error> { Self::new(s) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    FromStr for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    type Err = BoundedStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::new(s) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    Hash for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_str().hash(state) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    Display for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy>
    fmt::Debug for BoundedStr<MIN, MAX, MAX_BYTES, L, F>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundedStr")
            .field("value", &self.as_str())
            .field("len_bytes", &self.len_bytes())
            .field("len_logical", &self.len_logical())
            .finish()
    }
}

#[cfg(feature = "serde")]
impl<'de, const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy> 
    serde::Deserialize<'de> for BoundedStr<MIN, MAX, MAX_BYTES, L, F> 
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        
        Self::new(s).map_err(|e| {
            serde::de::Error::custom(match e {
                BoundedStrError::TooShort => "string too short",
                BoundedStrError::TooLong => "string too long",
                BoundedStrError::TooManyBytes => "too many bytes for buffer",
                BoundedStrError::InvalidContent => "invalid content format",
                BoundedStrError::MutationFailed => "mutation failed",
            })
        })
    }
}

#[cfg(feature = "serde")]
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy> 
    serde::Serialize for BoundedStr<MIN, MAX, MAX_BYTES, L, F> 
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}


pub type StackStr<const MIN: usize, const MAX: usize, const MAXB: usize = MAX, L = Bytes, F = AllowAll> =
    BoundedStr<MIN, MAX, MAXB, L, F>;

#[cfg(feature = "alloc")]
pub type FlexStr<const MIN: usize, const MAX: usize, const MAXB: usize = 4096, L = Bytes, F = AllowAll> =
    BoundedStr<MIN, MAX, MAXB, L, F>;