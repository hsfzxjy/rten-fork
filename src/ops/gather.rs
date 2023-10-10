use std::iter::zip;

use smallvec::SmallVec;
use wasnn_tensor::{Layout, SliceItem, Tensor, TensorView, View};

use crate::ops::reduce::{cmp_nan_greater, cmp_nan_less};
use crate::ops::{
    resolve_axis, resolve_index, Input, InputList, IntoOpResult, OpError, Operator, Output,
};

/// Gather elements from `input` specified by `indices`.
///
/// See <https://onnx.ai/onnx/operators/onnx__Gather.html>. Per the ONNX spec this
/// is very similar to `numpy.take`. See
/// <https://numpy.org/doc/stable/reference/generated/numpy.take.html> for
/// additional explanation.
pub fn gather<T: Copy + Default>(
    input: TensorView<T>,
    axis: isize,
    indices: TensorView<i32>,
) -> Result<Tensor<T>, OpError> {
    let axis = resolve_axis(input.ndim(), axis)?;

    for index in indices.iter().copied() {
        if index < 0 || index >= input.size(axis) as i32 {
            return Err(OpError::InvalidValue("Entry in `indices` is out of range"));
        }
    }

    let full_range =
        |ndim: usize| -> Vec<SliceItem> { (0..ndim).map(|_| SliceItem::RangeFull).collect() };

    // Fast path for scalar `indices`. This amounts to indexing `input` along
    // `axis`.
    if let (0, Some(index)) = (indices.ndim(), indices.item()) {
        let mut slice_range = full_range(input.ndim());
        slice_range[axis] = SliceItem::Index(*index as usize);
        let output = input.slice_dyn(&slice_range).to_tensor();
        return Ok(output);
    }

    let out_shape = [
        &input.shape()[..axis],
        indices.shape(),
        &input.shape()[axis + 1..],
    ]
    .concat();
    let mut output = Tensor::<T>::zeros(&out_shape);

    let mut in_range = full_range(input.ndim());
    let mut out_range = full_range(output.ndim());

    for (index_idx, index) in zip(indices.indices(), indices.iter()) {
        in_range[axis] = SliceItem::Index(*index as usize);
        for (i, index_val) in index_idx.into_iter().enumerate() {
            out_range[axis + i] = SliceItem::Index(index_val);
        }

        let in_slice = input.slice_dyn(&in_range);
        let mut out_slice = output.slice_mut_dyn(&out_range);
        out_slice.copy_from(&in_slice);
    }

    Ok(output)
}

#[derive(Debug)]
pub struct Gather {
    pub axis: isize,
}

impl Operator for Gather {
    fn name(&self) -> &str {
        "Gather"
    }

    fn run(&self, inputs: InputList) -> Result<Vec<Output>, OpError> {
        let input = inputs.require(0)?;
        let indices = inputs.require_as::<i32>(1)?;
        match input {
            Input::IntTensor(input) => {
                gather(input.view(), self.axis, indices.view()).into_op_result()
            }
            Input::FloatTensor(input) => {
                gather(input.view(), self.axis, indices.view()).into_op_result()
            }
        }
    }
}

// Specifies how to combine an existing element value with an update in a
// scatter operation.
#[derive(Copy, Clone, Debug)]
pub enum ScatterReduction {
    /// Add the existing value and update.
    Add,

    /// Multiply the existing value with the update.
    Mul,

    /// Take the minimum of the existing value and the update, propagating NaNs.
    Min,

    /// Take the maximum of the existing value and the update, propagating NaNs.
    Max,
}

pub fn scatter_elements<
    T: Copy + Default + PartialOrd + std::ops::Add<Output = T> + std::ops::Mul<Output = T>,
>(
    data: TensorView<T>,
    indices: TensorView<i32>,
    updates: TensorView<T>,
    axis: isize,
    reduction: Option<ScatterReduction>,
) -> Result<Tensor<T>, OpError> {
    if indices.ndim() != data.ndim() {
        return Err(OpError::InvalidValue(
            "`data` and `indices` must have same rank",
        ));
    }
    if indices.shape() != updates.shape() {
        return Err(OpError::InvalidValue(
            "`indices` and `updates` must have same shape",
        ));
    }
    let axis = resolve_axis(data.ndim(), axis)?;

    let reduce = |current, update| match reduction {
        Some(ScatterReduction::Add) => current + update,
        Some(ScatterReduction::Mul) => current * update,

        // nb. In the operations below, we prefer to keep the current value
        // unless the update is definitely less or NaN.
        Some(ScatterReduction::Min) => match cmp_nan_less(update, current) {
            std::cmp::Ordering::Less => update,
            _ => current,
        },
        Some(ScatterReduction::Max) => match cmp_nan_greater(update, current) {
            std::cmp::Ordering::Greater => update,
            _ => current,
        },
        None => update,
    };

    let mut output = data.to_tensor();
    for (index, update) in zip(updates.indices(), updates.iter()) {
        let target_index: SmallVec<[usize; 5]> = index
            .iter()
            .enumerate()
            .filter_map(|(dim, idx)| {
                if dim == axis {
                    resolve_index(data.size(dim), indices[&index] as isize)
                } else {
                    Some(*idx)
                }
            })
            .collect();
        if target_index.len() < data.ndim() {
            return Err(OpError::InvalidValue("Index is invalid"));
        }

        let out_el = &mut output[target_index];
        *out_el = reduce(*out_el, *update);
    }
    Ok(output)
}

#[derive(Debug)]
pub struct ScatterElements {
    pub axis: isize,
    pub reduction: Option<ScatterReduction>,
}

impl Operator for ScatterElements {
    fn name(&self) -> &str {
        "ScatterElements"
    }

    fn run(&self, inputs: InputList) -> Result<Vec<Output>, OpError> {
        let data = inputs.require(0)?;
        let indices = inputs.require_as::<i32>(1)?;
        let updates = inputs.require(2)?;

        match (data, updates) {
            (Input::IntTensor(data), Input::IntTensor(updates)) => scatter_elements(
                data.view(),
                indices.view(),
                updates.view(),
                self.axis,
                self.reduction,
            )
            .into_op_result(),
            (Input::FloatTensor(data), Input::FloatTensor(updates)) => scatter_elements(
                data.view(),
                indices.view(),
                updates.view(),
                self.axis,
                self.reduction,
            )
            .into_op_result(),
            _ => Err(OpError::IncorrectInputType),
        }
    }
}

#[cfg(test)]
mod tests {
    use wasnn_tensor::rng::XorShiftRng;
    use wasnn_tensor::test_util::expect_equal;
    use wasnn_tensor::{tensor, Layout, Tensor, View};

    use crate::ops::{gather, scatter_elements, OpError, ScatterReduction};

    #[test]
    fn test_gather_scalar_index() {
        // 1D input
        let input = tensor!([1, 20, 30]);
        for i in 0..input.len() {
            let indices = tensor!(i as i32);
            let result = gather(input.view(), 0, indices.view()).unwrap();
            assert_eq!(result.item(), Some(&input[[i]]))
        }

        // 2D input
        let input = tensor!((2, 2); [1, 2, 3, 4]);
        let result = gather(input.view(), 0, tensor!(0).view()).unwrap();
        assert_eq!(result, tensor!([1, 2]));
        let result = gather(input.view(), 0, tensor!(1).view()).unwrap();
        assert_eq!(result, tensor!([3, 4]));
    }

    #[test]
    fn test_gather() -> Result<(), String> {
        // Test case shrunk down from a small BERT model where `gather` is used
        // to lookup embeddings.
        let mut rng = XorShiftRng::new(1234);
        let input = Tensor::rand(&[128, 10], &mut rng);
        let indices = Tensor::from_data(&[2, 2], vec![2, 5, 8, 50]);
        let result = gather(input.view(), 0, indices.view()).unwrap();
        assert_eq!(result.shape(), &[2, 2, 10]);

        // Test case #1 from ONNX spec.
        let input = Tensor::from_data(&[3, 2], vec![1.0, 1.2, 2.3, 3.4, 4.5, 5.7]);
        let indices = Tensor::from_data(&[2, 2], vec![0, 1, 1, 2]);
        let expected = Tensor::from_data(&[2, 2, 2], vec![1.0, 1.2, 2.3, 3.4, 2.3, 3.4, 4.5, 5.7]);
        let result = gather(input.view(), 0, indices.view()).unwrap();
        expect_equal(&result, &expected)?;

        // Test case #2 from ONNX spec.
        let input = Tensor::from_data(&[3, 3], vec![1.0, 1.2, 1.9, 2.3, 3.4, 3.9, 4.5, 5.7, 5.9]);
        let indices = Tensor::from_data(&[1, 2], vec![0, 2]);
        let expected = Tensor::from_data(&[3, 1, 2], vec![1.0, 1.9, 2.3, 3.9, 4.5, 5.9]);
        let result = gather(input.view(), 1, indices.view()).unwrap();
        expect_equal(&result, &expected)
    }

    #[test]
    fn test_gather_invalid_inputs() {
        let mut rng = XorShiftRng::new(1234);
        let input = Tensor::rand(&[128, 10], &mut rng);
        let indices = Tensor::from_data(&[2, 2], vec![2, 5, 8, 50]);
        let result = gather(input.view(), 5, indices.view());
        assert_eq!(result.err(), Some(OpError::InvalidValue("Axis is invalid")));

        let indices = Tensor::from_data(&[2, 2], vec![2, 5, 8, 130]);
        let result = gather(input.view(), 0, indices.view());
        assert_eq!(
            result.err(),
            Some(OpError::InvalidValue("Entry in `indices` is out of range"))
        );
    }

    #[test]
    fn test_scatter_elements() {
        // Example #1 from ONNX spec
        let data = Tensor::zeros(&[3, 3]);
        let indices = tensor!((2, 3); [
            1, 0, 2, //
            0, 2, 1 //
        ]);
        let updates = tensor!((2, 3); [
            1., 1.1, 1.2, //
            2., 2.1, 2.2 //
        ]);
        let expected = tensor!((3, 3); [
            2., 1.1, 0., //
            1., 0., 2.2, //
            0., 2.1, 1.2 //
        ]);
        let result = scatter_elements(
            data.view(),
            indices.view(),
            updates.view(),
            0, /* axis */
            None,
        )
        .unwrap();
        assert_eq!(result, expected);

        // Example #2 from ONNX spec
        let data = tensor!((1, 5); [1., 2., 3., 4., 5.]);
        let indices = tensor!((1, 2); [1, 3]);
        let updates = tensor!((1, 2); [1.1, 2.1]);
        let expected = tensor!((1, 5); [
            1., 1.1, 3., 2.1, 5.
        ]);
        let result = scatter_elements(
            data.view(),
            indices.view(),
            updates.view(),
            1, /* axis */
            None,
        )
        .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_scatter_elements_reduction() {
        let data = tensor!([1, 2, 3, 4]);
        let indices = tensor!([1, 3]);
        let updates = tensor!([2, 2]);

        let scatter = |reduction: Option<ScatterReduction>| {
            scatter_elements(
                data.view(),
                indices.view(),
                updates.view(),
                0, /* axis */
                reduction,
            )
            .unwrap()
        };

        let result = scatter(Some(ScatterReduction::Add));
        assert_eq!(result, tensor!([1, 4, 3, 6]));

        let result = scatter(Some(ScatterReduction::Mul));
        assert_eq!(result, tensor!([1, 4, 3, 8]));

        let result = scatter(Some(ScatterReduction::Min));
        assert_eq!(result, tensor!([1, 2, 3, 2]));

        let result = scatter(Some(ScatterReduction::Max));
        assert_eq!(result, tensor!([1, 2, 3, 4]));
    }
}
