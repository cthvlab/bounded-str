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

enum Storage<const MAX_BYTES: usize> {
    Stack { buf: [u8; MAX_BYTES], len: usize },
    #[cfg(feature = "alloc")]
    Heap(Vec<u8>),
}

impl<const MAX_BYTES: usize> Clone for Storage<MAX_BYTES> {
    fn clone(&self) -> Self {
        match self {
            Self::Stack { buf, len } => Self::Stack { buf: *buf, len: *len },
            #[cfg(feature = "alloc")]
            Self::Heap(v) => Self::Heap(v.clone()),
        }
    }
}

pub struct BoundedStr<
    const MIN: usize,
    const MAX: usize,
    const MAX_BYTES: usize,
    L: LengthPolicy = Bytes,
    F: FormatPolicy = AllowAll,
	const Z: bool = false,
> {
    storage: Storage<MAX_BYTES>,
    _marker: PhantomData<(L, F, core::convert::Infallible)>, 
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    const _CHECK: () = {
        assert!(MIN <= MAX, "MIN must be <= MAX");
    };


    #[inline(always)]
	pub fn len_bytes(&self) -> usize {
        match &self.storage {
            Storage::Stack { len, .. } => *len,
            #[cfg(feature = "alloc")]
            Storage::Heap(v) => v.len(),
        }
    }

    #[inline(always)]
    pub fn len_logical(&self) -> usize {
        L::logical_len(self.as_str())
    }

    pub fn new(s: &str) -> Result<Self, BoundedStrError> {
        let logical_len = L::logical_len(s);
        if logical_len < MIN { return Err(BoundedStrError::TooShort); }
        if logical_len > MAX { return Err(BoundedStrError::TooLong); }
        if !F::check(s) { return Err(BoundedStrError::InvalidContent); }

        let byte_len = s.len();

        #[cfg(feature = "alloc")]
        if byte_len > MAX_BYTES {
            return Ok(Self {
                storage: Storage::Heap(s.as_bytes().to_vec()),
                _marker: PhantomData,
            });
        }

        if byte_len > MAX_BYTES {
            return Err(BoundedStrError::TooManyBytes);
        }

        let mut buf = [0u8; MAX_BYTES];
        buf[..byte_len].copy_from_slice(s.as_bytes());
        Ok(Self {
            storage: Storage::Stack { buf, len: byte_len },
            _marker: PhantomData,
        })
    }

    pub fn mutate<Mut, R>(&mut self, mutator: Mut) -> Result<R, BoundedStrError>
    where
        Mut: FnOnce(&mut [u8], &mut usize) -> R, 
    {
        match &mut self.storage {
            Storage::Stack { buf, len } => {
                let mut temp_buf = *buf;
                let mut temp_len = *len;
                let res = mutator(&mut temp_buf, &mut temp_len);
				
                if temp_len > MAX_BYTES { return Err(BoundedStrError::TooManyBytes); }

                if let Ok(s) = str::from_utf8(&temp_buf[..temp_len]) {
                    let l_len = L::logical_len(s);
                    
                    if l_len >= MIN && l_len <= MAX && F::check(s) {
                        *buf = temp_buf;
                        *len = temp_len;
                        return Ok(res);
                    }
                }
                Err(BoundedStrError::MutationFailed)
            }

            #[cfg(feature = "alloc")]            
            Storage::Heap(v) => {
                let mut temp_vec = v.clone();                
                let limit = core::cmp::max(MAX, MAX_BYTES);
                
                let old_len = temp_vec.len();

                if temp_vec.len() < limit {
                    temp_vec.resize(limit, 0); 
                }
                
                let mut temp_len = old_len;
                let res = mutator(&mut temp_vec, &mut temp_len);

                if temp_len > limit { 
                    Self::clear_temp_vec::<Z>(&mut temp_vec);
                    return Err(BoundedStrError::TooManyBytes); 
                }

                temp_vec.truncate(temp_len);
				
                if let Ok(s) = str::from_utf8(&temp_vec) {
                    let l_len = L::logical_len(s);
                    if l_len >= MIN && l_len <= MAX && F::check(s) {
                        *v = temp_vec;
                        return Ok(res);
                    }
                }

                Self::clear_temp_vec::<Z>(&mut temp_vec);
                Err(BoundedStrError::MutationFailed)
            }

        }
    }

    #[inline(always)]
	pub fn as_str(&self) -> &str {
        match &self.storage {
            Storage::Stack { buf, len } => unsafe { str::from_utf8_unchecked(&buf[..*len]) },
            #[cfg(feature = "alloc")]
            Storage::Heap(v) => unsafe { str::from_utf8_unchecked(v) },
        }
    }
	
	#[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.storage {
            Storage::Stack { buf, len } => &buf[..*len],
            #[cfg(feature = "alloc")]
            Storage::Heap(v) => v.as_slice(),
        }
    }
	
	#[cfg(feature = "constant-time")]
	#[inline(never)]
    fn constant_time_eq(&self, other: &[u8]) -> bool {
        let a = self.as_bytes();
        let b = other;

        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for i in 0..a.len() {            
            result |= a[i] ^ b[i];
        }
        result == 0
    }
	
	#[inline(always)]
    fn clear_temp_vec<const ZERO: bool>(v: &mut Vec<u8>) {
        #[cfg(feature = "zeroize")]
        if ZERO {
            for byte in v.iter_mut() {
                unsafe { core::ptr::write_volatile(byte, 0) };
            }
        }
    }
}


impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    PartialEq for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    fn eq(&self, other: &Self) -> bool {
        #[cfg(feature = "constant-time")]
        {
            self.constant_time_eq(other.as_bytes())
        }
        #[cfg(not(feature = "constant-time"))]
        {
            self.as_str() == other.as_str()
        }
    }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool> 
    Clone for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> {
    fn clone(&self) -> Self {
        Self { storage: self.storage.clone(), _marker: PhantomData }
    }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    Eq for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> {}
	
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    PartialEq<&str> for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    fn eq(&self, other: &&str) -> bool { self.as_str() == *other }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool> 
    Deref for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> {
    type Target = str;
    fn deref(&self) -> &str { self.as_str() }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    TryFrom<&str> for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    type Error = BoundedStrError;
    fn try_from(s: &str) -> Result<Self, Self::Error> { Self::new(s) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    FromStr for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    type Err = BoundedStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::new(s) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    Hash for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_str().hash(state) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    Display for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool>
    fmt::Debug for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundedStr")
            .field("value", &self.as_str())
            .field("len_bytes", &self.len_bytes())
            .field("len_logical", &self.len_logical())
            .finish()
    }
}

impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool> 
    Drop for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> 
{
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(feature = "zeroize")]
        if Z {
            match &mut self.storage {
                Storage::Stack { buf, .. } => {
                    for byte in buf.iter_mut() {
                        unsafe { core::ptr::write_volatile(byte, 0) };
                    }
                }
                #[cfg(feature = "alloc")]
                Storage::Heap(v) => {
                    for byte in v.iter_mut() {
                        unsafe { core::ptr::write_volatile(byte, 0) };
                    }
                }
            }
        }
    }
}


#[cfg(feature = "serde")]
impl<'de, const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool> 
    serde::Deserialize<'de> for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> 
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
impl<const MIN: usize, const MAX: usize, const MAX_BYTES: usize, L: LengthPolicy, F: FormatPolicy, const Z: bool> 
    serde::Serialize for BoundedStr<MIN, MAX, MAX_BYTES, L, F, Z> 
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}


pub type StackStr<const MIN: usize, const MAX: usize, const MAXB: usize = MAX, L = Bytes, F = AllowAll, const Z: bool = false > = BoundedStr<MIN, MAX, MAXB, L, F, Z>;

#[cfg(feature = "alloc")]
pub type FlexStr<const MIN: usize, const MAX: usize, const MAXB: usize = 4096, L = Bytes, F = AllowAll, const Z: bool = false > = BoundedStr<MIN, MAX, MAXB, L, F, Z>;