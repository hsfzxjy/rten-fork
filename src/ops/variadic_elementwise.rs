use std::cmp::Ordering;

use rten_tensor::prelude::*;
use rten_tensor::{Tensor, TensorView};

use crate::ops::binary_elementwise::binary_op;
use crate::ops::reduce::{cmp_nan_greater, cmp_nan_less};
use crate::ops::{Input, InputList, IntoOpResult, OpError, Operator, OutputList};
use crate::tensor_pool::{AutoReturn, TensorPool};

/// Apply an elementwise reduction to a sequence of tensors.
///
/// All inputs must be broadcastable to the same shape.
fn reduce_elementwise<T: Copy, R: Fn(T, T) -> T + Copy>(
    pool: &TensorPool,
    inputs: &[TensorView<T>],
    reduce: R,
) -> Result<Tensor<T>, OpError> {
    match inputs {
        [] => Err(OpError::InvalidValue("Expected at least one input")),
        [a] => Ok(a.to_tensor_in(pool)),
        [a, b] => binary_op(pool, a.view(), b.view(), reduce),
        [a, b, c @ ..] => {
            let mut tmp = binary_op(pool, a.view(), b.view(), reduce)?;
            for arg in c {
                let old_tmp = tmp.auto_return(pool);
                tmp = binary_op(pool, old_tmp.view(), arg.view(), reduce)?;
            }
            Ok(tmp)
        }
    }
}

/// Extract operator inputs as a vec of tensor views of the same type.
fn typed_views<'a, T>(inputs: &'a InputList) -> Result<Vec<TensorView<'a, T>>, OpError>
where
    Input<'a>: TryInto<TensorView<'a, T>, Error = OpError>,
{
    inputs.iter().try_fold(Vec::new(), |mut acc, input| {
        acc.push(input.try_into()?);
        Ok(acc)
    })
}

pub fn max<T: Copy + PartialOrd>(
    pool: &TensorPool,
    inputs: &[TensorView<T>],
) -> Result<Tensor<T>, OpError> {
    reduce_elementwise(pool, inputs, |a, b| match cmp_nan_greater(a, b) {
        Ordering::Equal | Ordering::Greater => a,
        Ordering::Less => b,
    })
}

macro_rules! run_typed_op {
    ($pool:expr, $inputs:ident, $op:ident) => {{
        let first = $inputs.require(0)?;
        match first {
            Input::FloatTensor(_) => {
                let inputs: Vec<TensorView<f32>> = typed_views(&$inputs)?;
                $op($pool, &inputs).into_op_result()
            }
            Input::Int32Tensor(_) => {
                let inputs: Vec<TensorView<i32>> = typed_views(&$inputs)?;
                $op($pool, &inputs).into_op_result()
            }
            _ => Err(OpError::UnsupportedType),
        }
    }};
}

#[derive(Debug)]
pub struct Max {}

impl Operator for Max {
    fn name(&self) -> &str {
        "Max"
    }

    fn run(&self, pool: &TensorPool, inputs: InputList) -> Result<OutputList, OpError> {
        run_typed_op!(pool, inputs, max)
    }
}

pub fn mean(pool: &TensorPool, inputs: &[TensorView]) -> Result<Tensor, OpError> {
    let mut result = sum(pool, inputs)?;
    result.apply(|x| x / inputs.len() as f32);
    Ok(result)
}

#[derive(Debug)]
pub struct Mean {}

impl Operator for Mean {
    fn name(&self) -> &str {
        "Mean"
    }

    fn run(&self, pool: &TensorPool, inputs: InputList) -> Result<OutputList, OpError> {
        let inputs: Vec<TensorView<f32>> = typed_views(&inputs)?;
        mean(pool, &inputs).into_op_result()
    }
}

pub fn min<T: Copy + PartialOrd>(
    pool: &TensorPool,
    inputs: &[TensorView<T>],
) -> Result<Tensor<T>, OpError> {
    reduce_elementwise(pool, inputs, |a, b| match cmp_nan_less(a, b) {
        Ordering::Less | Ordering::Equal => a,
        Ordering::Greater => b,
    })
}

#[derive(Debug)]
pub struct Min {}

impl Operator for Min {
    fn name(&self) -> &str {
        "Min"
    }

    fn run(&self, pool: &TensorPool, inputs: InputList) -> Result<OutputList, OpError> {
        run_typed_op!(pool, inputs, min)
    }
}

pub fn sum<T: Copy + std::ops::Add<Output = T>>(
    pool: &TensorPool,
    inputs: &[TensorView<T>],
) -> Result<Tensor<T>, OpError> {
    reduce_elementwise(pool, inputs, |a, b| a + b)
}

#[derive(Debug)]
pub struct Sum {}

impl Operator for Sum {
    fn name(&self) -> &str {
        "Sum"
    }

    fn run(&self, pool: &TensorPool, inputs: InputList) -> Result<OutputList, OpError> {
        run_typed_op!(pool, inputs, sum)
    }
}

#[cfg(test)]
mod tests {
    use rten_tensor::prelude::*;
    use rten_tensor::test_util::eq_with_nans;
    use rten_tensor::{Tensor, TensorView};

    use crate::ops::tests::new_pool;
    use crate::ops::{max, mean, min, sum, Input, InputList, Max, Min, OpError, Operator, Sum};

    fn run_operator<Op: Operator>(op: &Op, inputs: &[TensorView]) -> Tensor {
        let inputs: Vec<Input> = inputs.iter().cloned().map(|i| i.into()).collect();
        let pool = new_pool();
        let mut outputs = op.run(&pool, InputList::from(inputs.as_slice())).unwrap();
        outputs.remove(0).try_into().unwrap()
    }

    // nb. Most of the tests are written for the `max` operator only, as the
    // other elementwise reductions share most of the implementation.
    #[test]
    fn test_max() {
        struct Case {
            inputs: Vec<Tensor>,
            expected: Result<Tensor, OpError>,
        }

        let cases = [
            // Zero inputs
            Case {
                inputs: vec![],
                expected: Err(OpError::InvalidValue("Expected at least one input")),
            },
            // One input
            Case {
                inputs: vec![[1., 2., 3., 4.].into()],
                expected: Ok([1., 2., 3., 4.].into()),
            },
            // Two inputs
            Case {
                inputs: vec![[1., 2., 3.].into(), [4., 1., 3.].into()],
                expected: Ok([4., 2., 3.].into()),
            },
            // Two inputs with NaNs
            Case {
                inputs: vec![[1., 2., f32::NAN].into(), [4., 1., 3.].into()],
                expected: Ok([4., 2., f32::NAN].into()),
            },
            // Three inputs
            Case {
                inputs: vec![[1., 2.].into(), [5., 1.].into(), [2., 3.].into()],
                expected: Ok([5., 3.].into()),
            },
            // Two inputs, broadcasted
            Case {
                inputs: vec![[2., 4.].into(), [[1., 2.], [3., 4.]].into()],
                expected: Ok([[2., 4.], [3., 4.]].into()),
            },
            // Three inputs, broadcasted
            Case {
                inputs: vec![
                    [2., 4.].into(),
                    Tensor::from(3.),
                    [[1., 2.], [3., 4.]].into(),
                ],
                expected: Ok(Tensor::from([[3., 4.], [3., 4.]])),
            },
            // Two inputs, incompatible broadcast
            Case {
                inputs: vec![[4., 5., 6.].into(), [[1., 2.], [3., 4.]].into()],
                expected: Err(OpError::IncompatibleInputShapes("Cannot broadcast inputs")),
            },
            // Three inputs, incompatible broadcast
            Case {
                inputs: vec![
                    [2., 4., 5.].into(),
                    Tensor::from(3.),
                    [[1., 2.], [3., 4.]].into(),
                ],
                expected: Err(OpError::IncompatibleInputShapes("Cannot broadcast inputs")),
            },
        ];

        let pool = new_pool();
        for case in cases {
            let views: Vec<_> = case.inputs.iter().map(|t| t.view()).collect();
            let result = max(&pool, &views);
            match (result, case.expected) {
                (Ok(result), Ok(expected)) => assert!(eq_with_nans(result.view(), expected.view())),
                (result, expected) => assert_eq!(result, expected),
            }
        }

        // Test the `Max` Operator impl
        let a = Tensor::from([1., 2., 7., 8.]);
        let b = Tensor::from([5., 6., 3., 4.]);
        let expected = Tensor::from([5., 6., 7., 8.]);
        let op_result = run_operator(&Max {}, &[a.view(), b.view()]);
        assert_eq!(op_result, expected);
    }

    #[test]
    fn test_mean() {
        let a = Tensor::from([1., 2., 3., 4.]);
        let b = Tensor::from([5., 6., 7., 8.]);
        let pool = new_pool();
        assert_eq!(
            mean(&pool, &[a.view(), b.view()]),
            Ok(Tensor::from([3., 4., 5., 6.]))
        );
    }

    #[test]
    fn test_min() {
        let pool = new_pool();

        let (a, b) = (Tensor::from([1., 2., 3.]), Tensor::from([4., 1., 3.]));
        let expected = Tensor::from([1., 1., 3.]);
        assert_eq!(min(&pool, &[a.view(), b.view()]), Ok(expected.clone()));

        let output = run_operator(&Min {}, &[a.view(), b.view()]);
        assert_eq!(output, expected);

        let (a, b) = (Tensor::from([1., 2., f32::NAN]), Tensor::from([4., 1., 3.]));
        let result = min(&pool, &[a.view(), b.view()]).unwrap();
        assert!(eq_with_nans(
            result.view(),
            Tensor::from([1., 1., f32::NAN]).view()
        ));
    }

    #[test]
    fn test_sum() {
        let pool = new_pool();
        let a = Tensor::from([1., 2., 3., 4.]);
        let b = Tensor::from([5., 6., 7., 8.]);
        let expected = Tensor::from([6., 8., 10., 12.]);

        assert_eq!(sum(&pool, &[a.view(), b.view()]), Ok(expected.clone()));

        let output = run_operator(&Sum {}, &[a.view(), b.view()]);
        assert_eq!(output, expected);
    }
}
