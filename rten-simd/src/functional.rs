//! Higher order functions (map, fold etc.) that use vectorized operations.

use std::mem::MaybeUninit;

use crate::span::{MutPtrLen, PtrLen};
use crate::SimdFloat;

/// Apply a unary operation to each element in `input` and store the results
/// in `output`.
///
/// When evaluated, all elements in `output` will be initialized.
///
/// The operation is applied to SIMD vector-sized groups of elements at a
/// time using `op`. If the final group has a size that is smaller than the
/// SIMD vector width, `op` will be called with a SIMD vector that is
/// padded.
///
/// # Safety
///
/// The caller must ensure that `S` is a supported SIMD vector type on the
/// current system.
#[inline(always)]
pub unsafe fn simd_map<S: SimdFloat, Op: FnMut(S) -> S>(
    input: PtrLen<f32>,
    output: MutPtrLen<MaybeUninit<f32>>,
    mut op: Op,
    pad: f32,
) {
    assert!(input.len() == output.len());

    let mut n = input.len();
    let mut in_ptr = input.ptr();
    let mut out_ptr = output.ptr();

    while n >= S::LEN {
        let x = S::load(in_ptr);
        let y = op(x);
        y.store(out_ptr as *mut f32);

        n -= S::LEN;
        in_ptr = in_ptr.add(S::LEN);
        out_ptr = out_ptr.add(S::LEN);
    }

    if n > 0 {
        let x = S::load_partial(in_ptr, n, pad);
        let y = op(x);
        y.store_partial(out_ptr as *mut f32, n);
    }
}

/// Apply a vectorized fold operation over `xs`. If the length of `xs` is not
/// a multiple of `S::LEN` then the final update will use a vector padded
/// with `pad`.
///
/// # Safety
///
/// The caller must ensure that `S` is a supported SIMD vector type on the
/// current system.
#[inline(always)]
pub unsafe fn simd_fold<S: SimdFloat, Op: Fn(S, S) -> S>(
    xs: PtrLen<f32>,
    mut accum: S,
    simd_op: Op,
    pad: f32,
) -> S {
    let mut n = xs.len();
    let mut x_ptr = xs.ptr();

    while n >= S::LEN {
        let x = S::load(x_ptr);
        accum = simd_op(accum, x);
        n -= S::LEN;
        x_ptr = x_ptr.add(S::LEN);
    }

    if n > 0 {
        let x = S::load_partial(x_ptr, n, pad);
        accum = simd_op(accum, x);
    }

    accum
}
