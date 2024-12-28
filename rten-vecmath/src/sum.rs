use rten_simd::dispatch::{dispatch, SimdOp};
use rten_simd::functional::simd_fold;
use rten_simd::SimdFloat;

struct Sum<'a> {
    input: &'a [f32],
}

impl SimdOp for Sum<'_> {
    type Output = f32;

    #[inline(always)]
    unsafe fn eval<S: SimdFloat>(self) -> Self::Output {
        let vec_sum = simd_fold(
            self.input.into(),
            S::zero(),
            #[inline(always)]
            |sum, x| sum.add(x),
        );
        vec_sum.sum()
    }
}

/// Compute the sum of a slice of floats.
///
/// This is more efficient than `xs.iter().sum()` as it computes multiple
/// partial sums in parallel using SIMD and then sums across the SIMD lanes at
/// the end. This will produce very slightly different results because the
/// additions are happening in a different order.
pub fn sum(xs: &[f32]) -> f32 {
    let op = Sum { input: xs };
    dispatch(op)
}

struct SumSquare<'a> {
    input: &'a [f32],
}

impl SimdOp for SumSquare<'_> {
    type Output = f32;

    #[inline(always)]
    unsafe fn eval<S: SimdFloat>(self) -> Self::Output {
        let vec_sum = simd_fold(
            self.input.into(),
            S::zero(),
            #[inline(always)]
            |sum, x| x.mul_add(x, sum),
        );
        vec_sum.sum()
    }
}

/// Compute the sum of the squares of elements in `xs`.
///
/// Conceptually this is like `xs.iter().map(|&x| x * x).sum()` but more
/// efficient as it computes multiple partial sums in parallel and then sums
/// across SIMD lanes at the end. The results will also be slightly different
/// because the additions are happening in a different order.
pub fn sum_square(xs: &[f32]) -> f32 {
    let op = SumSquare { input: xs };
    dispatch(op)
}

struct SumSquareSub<'a> {
    input: &'a [f32],
    offset: f32,
}

impl SimdOp for SumSquareSub<'_> {
    type Output = f32;

    #[inline(always)]
    unsafe fn eval<S: SimdFloat>(self) -> Self::Output {
        let offset_vec = S::splat(self.offset);
        let vec_sum = simd_fold(
            self.input.into(),
            S::zero(),
            #[inline(always)]
            |sum, x| {
                let x_offset = x.sub(offset_vec);
                x_offset.mul_add(x_offset, sum)
            },
        );
        vec_sum.sum()
    }
}

/// Compute the sum of squares of `xs - offset`.
///
/// This is a variant of [`sum`] which subtracts a constant value from each
/// element before squaring it. A typical use case is to compute the variance of
/// a sequence, which is defined as `mean((X - x_mean)^2)`.
pub fn sum_square_sub(xs: &[f32], offset: f32) -> f32 {
    let op = SumSquareSub { input: xs, offset };
    dispatch(op)
}

#[cfg(test)]
mod tests {
    use super::{sum, sum_square, sum_square_sub};

    // Chosen to not be a multiple of vector size, so that tail handling is
    // exercised.
    const LEN: usize = 100;

    #[test]
    fn test_sum() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let expected_sum: f32 = xs.iter().sum();
        let sum = sum(&xs);
        assert_eq!(sum, expected_sum);
    }

    #[test]
    fn test_sum_square() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let expected_sum: f32 = xs.iter().copied().map(|x| x * x).sum();
        let sum = sum_square(&xs);
        assert_eq!(sum, expected_sum);
    }

    #[test]
    fn test_sum_square_sub() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let mean = xs.iter().sum::<f32>() / xs.len() as f32;
        let expected_sum: f32 = xs.iter().copied().map(|x| (x - mean) * (x - mean)).sum();
        let sum = sum_square_sub(&xs, mean);
        assert_eq!(sum, expected_sum);
    }
}
