//! Byte manipulation utilities.
//!
//! This file contains utility macros
//! for manipulating and retrieving
//! parts of multi-byte data.

use num_traits::{PrimInt, one, zero};

/// Create a range of consecutive 1-bits.
///
/// Creates a number which in binary corresponds to
/// a range of consecutive bits set to 1 and the
/// rest set to 0.
#[inline(always)]
pub const fn CONST_BIT_RANGE<T: PrimInt>(start: u8, end: u8) -> T {
    !(!zero::<T>() << (end - start)) << start
}

/// Get the bit at a particular position in a word.
#[inline(always)]
pub const fn GET_BIT<T: PrimInt>(word: T, pos: u8) -> bool {
    word & (one() << pos) != 0
}

/// Reduces a value to have length many bits.
///
/// Any bits after length are zeroed out.
#[inline(always)]
pub const fn TRIM_BITS<T: PrimInt>(word: T, length: u8) -> T {
    word & CONST_BIT_RANGE(0, length)
}

/// Zeroes out all bits but those within a range.
///
/// Obtains a range of bits in a value, but keeps them at their original
/// position in the value, zeroing the rest.
#[inline(always)]
pub const fn FILTER_BIT_RANGE<T: PrimInt>(word: T, start: u8, end: u8) -> T {
    word & CONST_BIT_RANGE(start, end)
}

/// Extracts the bits within a range.
///
/// Obtains a range of bits in a value, shifting them to the right
/// so that only the value represented by that range is present.
#[inline(always)]
pub const fn GET_BIT_RANGE<T: PrimInt>(word: T, start: u8, end: u8) -> T {
    TRIM_BITS(word >> start, end - start)
}

/// Overwrites a bit with a value.
///
/// Replaces a particular bit in a word with a new value.
/// The value is trimmed so only its rightmost bit is used.
#[inline(always)]
pub const fn UPDATE_BIT<T: PrimInt>(word: T, pos: u8, val: T) {
    (word & !(one() << pos)) | (TRIM_BITS(val, 1) << pos)
}

/// Overwrites a range of bits with a value.
///
/// Replaces a range of bits in a word with a new value.
/// The value is trimmed so that only the necessary rightmost bits
/// are used.
#[inline(always)]
pub const fn UPDATE_BIT_RANGE<T: PrimInt>(word: T, start: u8, end: u8, val: T) {
    (word & !CONST_BIT_RANGE(start, end)) | (TRIM_BITS(val, end - start) << start)
}

/// Calculates the number of bits in a type.
#[inline(always)]
pub const fn BIT_SIZE_OF<T>() -> usize {
    size_of::<T>() * 8
}

pub const SIZE_B: u8 = 8;
pub const SIZE_W: u8 = 16;
pub const SIZE_D: u8 = 32;

/// Extracts the most significant byte of a 16 bit word.
#[inline(always)]
pub const fn MSB(d: u16) -> u8 {
    GET_BIT_RANGE(d, SIZE_B, 2 * SIZE_B) as u8
}

/// Extracts the least significant byte of a 16 bit word.
#[inline(always)]
pub const fn LSB(d: u16) -> u8 {
    GET_BIT_RANGE(d, 0, SIZE_B) as u8
}
