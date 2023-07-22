use std::borrow::Cow;
use std::fmt::Debug;
use std::io;
use std::iter::zip;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

use crate::iterators::{AxisIter, AxisIterMut, BroadcastIter, Iter, IterMut, Offsets};
use crate::layout::{DynLayout, Layout};
use crate::ndtensor::{NdTensorBase, NdTensorView, NdTensorViewMut};
use crate::range::{IntoSliceItems, SliceItem, SliceRange};
use crate::rng::XorShiftRng;

/// Trait for indexing a `Tensor`
pub trait TensorIndex {
    /// Return the number of dimensions in the index.
    fn len(&self) -> usize;

    /// Return true if this index has zero dimensions (ie. is a scalar).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the index for dimension `dim`
    fn index(&self, dim: usize) -> usize;
}

impl<Array: AsRef<[usize]>> TensorIndex for Array {
    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn index(&self, dim: usize) -> usize {
        self.as_ref()[dim]
    }
}

/// N-dimensional array, where `N` is determined at runtime based on the shape
/// that is specified when the tensor is constructed.
///
/// `T` is the element type and `S` is the element storage.
///
/// Most callers will not use `TensorBase` directly but instead use the type
/// aliases [Tensor], [TensorView] and [TensorViewMut]. [Tensor] owns
/// its elements, and the other two types are views of slices.
///
/// [TensorBase] implements the [Layout] trait which provides information about
/// the shape and strides of the tensor.
#[derive(Debug)]
pub struct TensorBase<T, S: AsRef<[T]>> {
    data: S,
    layout: DynLayout,
    element_type: PhantomData<T>,
}

/// TensorView provides a view onto data owned by a [Tensor].
///
/// Conceptually the relationship between TensorView and Tensor is similar to
/// that between slice and Vec. They share the same element buffer, but views
/// can have distinct layouts, with some limitations.
pub type TensorView<'a, T = f32> = TensorBase<T, &'a [T]>;

/// TensorViewMut provides a mutable view onto data owned by a [Tensor].
///
/// This is similar to [TensorView], except elements in the underyling
/// Tensor can be modified through it.
pub type TensorViewMut<'a, T = f32> = TensorBase<T, &'a mut [T]>;

/// Trait for sources of random data for tensors.
pub trait RandomSource<T> {
    fn next(&mut self) -> T;
}

impl<T, S: AsRef<[T]>> TensorBase<T, S> {
    /// Create a new tensor with a given layout and storage.
    pub(crate) fn new(data: S, layout: &DynLayout) -> Self {
        TensorBase {
            data,
            layout: layout.clone(),
            element_type: PhantomData,
        }
    }

    /// Create a new tensor from a given shape and set of elements. No copying
    /// is required.
    pub fn from_data<D: Into<S>>(shape: &[usize], data: D) -> Self {
        let data = data.into();
        assert!(
            shape[..].iter().product::<usize>() == data.as_ref().len(),
            "Number of elements given by shape {:?} does not match data length {}",
            shape,
            data.as_ref().len()
        );
        TensorBase {
            data,
            layout: DynLayout::new(shape),
            element_type: PhantomData,
        }
    }

    /// Consume self and return the underlying element buffer.
    ///
    /// As with [TensorBase::data], there is no guarantee about the ordering of
    /// elements.
    pub fn into_data(self) -> S {
        self.data
    }

    /// Return a copy of this tensor with each element replaced by `f(element)`.
    ///
    /// The order in which elements are visited is unspecified and may not
    /// correspond to the logical order.
    pub fn map<F, U>(&self, f: F) -> Tensor<U>
    where
        F: Fn(&T) -> U,
    {
        let data = self.view().iter().map(f).collect();
        Tensor {
            data,
            layout: DynLayout::new(self.shape()),
            element_type: PhantomData,
        }
    }

    /// Return an immutable view of this tensor.
    ///
    /// Views share the same element array, but can have an independent layout,
    /// with some limitations.
    pub fn view(&self) -> TensorView<T> {
        TensorView::new(self.data.as_ref(), &self.layout)
    }

    /// Change the layout to put dimensions in the order specified by `dims`.
    ///
    /// This does not modify the order of elements in the data buffer, it just
    /// updates the strides used by indexing.
    pub fn permute(&mut self, dims: &[usize]) {
        self.layout.permute(dims);
    }

    /// Move the index at axis `from` to `to`, keeping the relative order of
    /// other dimensions the same. This is like NumPy's `moveaxis` function.
    ///
    /// Panics if the `from` or `to` axes are >= `self.ndim()`.
    pub fn move_axis(&mut self, from: usize, to: usize) {
        self.layout.move_axis(from, to);
    }

    /// Reverse the order of dimensions.
    ///
    /// This does not modify the order of elements in the data buffer, it just
    /// changes the strides used by indexing.
    pub fn transpose(&mut self) {
        self.layout.transpose();
    }

    /// Insert a dimension of size one at index `dim`.
    pub fn insert_dim(&mut self, dim: usize) {
        self.layout.insert_dim(dim);
    }

    /// Return the layout which maps indices to offsets in the data.
    pub fn layout(&self) -> &DynLayout {
        &self.layout
    }

    /// Return an iterator over offsets of elements in this tensor, in their
    /// logical order.
    ///
    /// See also the notes for `slice_offsets`.
    #[cfg(test)]
    fn offsets(&self) -> Offsets {
        Offsets::new(self.layout())
    }

    /// Return an iterator over offsets of this tensor, broadcasted to `shape`.
    ///
    /// This is very similar to `broadcast_iter`, except that the iterator
    /// yields offsets into rather than elements of the data buffer.
    pub fn broadcast_offsets(&self, shape: &[usize]) -> Offsets {
        assert!(
            self.can_broadcast_to(shape),
            "Cannot broadcast to specified shape"
        );
        Offsets::broadcast(self.layout(), shape)
    }

    /// Return an iterator over offsets of elements in this tensor.
    ///
    /// Note that the offset order of the returned iterator will become incorrect
    /// if the tensor's layout is modified during iteration.
    pub fn slice_offsets(&self, ranges: &[SliceRange]) -> Offsets {
        Offsets::slice(self.layout(), ranges)
    }

    /// Return a new contiguous tensor with the same shape and elements as this
    /// view.
    pub fn to_owned(&self) -> Tensor<T>
    where
        T: Clone,
    {
        Tensor::from_data(
            self.shape(),
            self.view().iter().cloned().collect::<Vec<_>>(),
        )
    }

    /// Return a copy of the elements of this tensor as a contiguous vector
    /// in row-major order.
    ///
    /// This is slightly more efficient than `iter().collect()` in the case
    /// where the tensor is already contiguous.
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        if self.is_contiguous() {
            self.data.as_ref().to_vec()
        } else {
            self.view().iter().cloned().collect()
        }
    }
}

impl<'a, T> TensorBase<T, &'a [T]> {
    /// Return an iterator over slices of this tensor along a given axis.
    pub fn axis_iter(&self, dim: usize) -> AxisIter<'a, T> {
        AxisIter::new(self, dim)
    }

    /// Return an iterator over elements of this tensor, broadcasted to `shape`.
    ///
    /// A broadcasted iterator behaves as if the tensor had the broadcasted
    /// shape, repeating elements as necessary to fill the given dimensions.
    /// Broadcasting is only possible if the actual and broadcast shapes are
    /// compatible according to ONNX's rules. See
    /// <https://github.com/onnx/onnx/blob/main/docs/Operators.md>.
    ///
    /// See also <https://numpy.org/doc/stable/user/basics.broadcasting.html#general-broadcasting-rules>
    /// for worked examples of how broadcasting works.
    pub fn broadcast_iter(&self, shape: &[usize]) -> BroadcastIter<'a, T> {
        assert!(
            self.can_broadcast_to(shape),
            "Cannot broadcast to specified shape"
        );
        BroadcastIter::new(self, shape)
    }

    /// Return the element buffer for this tensor as a slice.
    ///
    /// If the tensor is contiguous, the buffer will contain the same elements
    /// in the same order as yielded by [Tensor::iter]. In other cases the buffer
    /// may have unused indexes or a different ordering.
    pub fn data(&self) -> &'a [T] {
        self.data
    }

    /// Returns the single item if this tensor is a 0-dimensional tensor
    /// (ie. a scalar)
    pub fn item(&self) -> Option<&'a T> {
        match self.ndim() {
            0 => Some(&self.data[0]),
            _ if self.len() == 1 => self.iter().next(),
            _ => None,
        }
    }

    /// Return an iterator over elements of this tensor, in their logical order.
    pub fn iter(&self) -> Iter<'a, T> {
        Iter::new(self)
    }

    /// Return an `NdTensor` version of this view.
    ///
    /// Panics if the rank of this tensor is not `N`.
    pub fn nd_view<const N: usize>(&self) -> NdTensorView<'a, T, N> {
        self.nd_slice([])
    }

    /// Return an N-dimensional view of a slice of this tensor.
    ///
    /// See notes in [TensorBase::nd_view].
    ///
    /// Base specifies zero or more indices to slice the view with, and N
    /// is the rank of the returned view. `B + N` must equal `self.ndim()`.
    pub fn nd_slice<const B: usize, const N: usize>(
        &self,
        base: [usize; B],
    ) -> NdTensorView<'a, T, N> {
        assert!(B + N == self.ndim());
        let offset = self.layout.slice_offset(base);
        let data = &self.data()[offset..];
        let strides = self.layout.strides()[self.ndim() - N..].try_into().unwrap();
        let shape = self.layout.shape()[self.ndim() - N..].try_into().unwrap();
        NdTensorView::from_slice(data, shape, Some(strides)).unwrap()
    }

    /// Return a new view with the dimensions re-ordered according to `dims`.
    pub fn permuted(&self, dims: &[usize]) -> TensorView<'a, T> {
        Self {
            data: self.data,
            layout: self.layout.permuted(dims),
            element_type: PhantomData,
        }
    }

    /// Return a new view with a given shape. This has the same requirements
    /// as `reshape`.
    pub fn reshaped(&self, shape: &[usize]) -> TensorView<'a, T> {
        Self {
            data: self.data,
            layout: self.layout.reshaped(shape),
            element_type: PhantomData,
        }
    }

    /// Change the layout of this view to have the given shape.
    ///
    /// The current view must be contiguous and the new shape must have the
    /// same product as the current shape.
    pub fn reshape(&mut self, shape: &[usize]) {
        self.layout.reshape(shape);
    }

    /// Return a view of part of this tensor.
    ///
    /// `range` specifies the indices or ranges of this tensor to include in the
    /// returned view. If `N` is less than the number of dimensions in this
    /// tensor, `range` refers to the leading dimensions, and is padded to
    /// include the full range of the remaining dimensions.
    pub fn slice<const N: usize, R: IntoSliceItems<N>>(&self, range: R) -> TensorView<'a, T> {
        self.slice_dyn(&range.into_slice_items())
    }

    /// Return a view of part of this tensor.
    ///
    /// This is like [TensorBase::slice] but supports a dynamic number of
    /// slice items.
    pub fn slice_dyn(&self, range: &[SliceItem]) -> TensorView<'a, T> {
        let (offset_range, layout) = self.layout.slice(range);
        TensorBase {
            data: &self.data[offset_range],
            layout,
            element_type: PhantomData,
        }
    }

    /// Return an iterator over a slice of this tensor.
    pub fn slice_iter(&self, ranges: &[SliceRange]) -> Iter<'a, T> {
        Iter::slice(self, ranges)
    }

    /// Return a view of this tensor with all dimensions of size 1 removed.
    pub fn squeezed(&self) -> TensorView<'a, T> {
        TensorBase {
            data: self.data,
            layout: self.layout.squeezed(),
            element_type: PhantomData,
        }
    }

    /// Return a tensor with data laid out in contiguous order. This will
    /// be a view if the data is already contiguous, or a copy otherwise.
    pub fn to_contiguous(&self) -> TensorBase<T, Cow<'a, [T]>>
    where
        T: Clone,
    {
        if self.is_contiguous() {
            TensorBase {
                data: Cow::Borrowed(self.data),
                layout: self.layout.clone(),
                element_type: PhantomData,
            }
        } else {
            let data: Vec<T> = self.iter().cloned().collect();
            TensorBase {
                data: Cow::Owned(data),
                layout: DynLayout::new(self.layout().shape()),
                element_type: PhantomData,
            }
        }
    }

    /// Return a new view with the order of dimensions reversed.
    pub fn transposed(&self) -> TensorView<'a, T> {
        Self {
            data: self.data,
            layout: self.layout.transposed(),
            element_type: PhantomData,
        }
    }
}

impl<T, S: AsRef<[T]>> Layout for TensorBase<T, S> {
    type Index<'a> = <DynLayout as Layout>::Index<'a> where S: 'a, T: 'a;
    type Indices = <DynLayout as Layout>::Indices;

    /// Return the number of dimensions.
    fn ndim(&self) -> usize {
        self.layout.ndim()
    }

    /// Returns the number of elements in the array.
    fn len(&self) -> usize {
        self.layout.len()
    }

    /// Returns true if the array has no elements.
    fn is_empty(&self) -> bool {
        self.layout.is_empty()
    }

    /// Returns an array of the sizes of each dimension.
    fn shape(&self) -> Self::Index<'_> {
        self.layout.shape()
    }

    /// Returns the size of the dimension `dim`.
    fn size(&self, dim: usize) -> usize {
        self.layout.size(dim)
    }

    /// Returns an array of the strides of each dimension.
    fn strides(&self) -> Self::Index<'_> {
        self.layout.strides()
    }

    /// Returns the offset between adjacent indices along dimension `dim`.
    fn stride(&self, dim: usize) -> usize {
        self.layout.stride(dim)
    }

    /// Return an iterator over all valid indices in this tensor.
    fn indices(&self) -> Self::Indices {
        self.layout.indices()
    }
}

impl<I: TensorIndex, T, S: AsRef<[T]>> Index<I> for TensorBase<T, S> {
    type Output = T;
    fn index(&self, index: I) -> &Self::Output {
        &self.data.as_ref()[self.layout.offset(index)]
    }
}

impl<T, S: AsRef<[T]> + AsMut<[T]>> TensorBase<T, S> {
    /// Copy elements from another tensor into this tensor.
    ///
    /// This tensor and `other` must have the same shape.
    pub fn copy_from(&mut self, other: &TensorView<T>)
    where
        T: Clone,
    {
        assert!(self.shape() == other.shape());
        for (out, x) in zip(self.iter_mut(), other.iter()) {
            *out = x.clone();
        }
    }

    /// Return the element buffer for this tensor as a mutable slice.
    ///
    /// WARNING: See notes about ordering in [TensorBase::data].
    pub fn data_mut(&mut self) -> &mut [T] {
        self.data.as_mut()
    }

    /// Return a mutable iterator over elements of this view.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        let layout = &self.layout;
        IterMut::new(self.data.as_mut(), layout)
    }

    /// Return an iterator over mutable slices of this tensor along a given
    /// axis.
    pub fn axis_iter_mut(&mut self, dim: usize) -> AxisIterMut<T> {
        AxisIterMut::new(self.view_mut(), dim)
    }

    /// Replace elements of this tensor with `f(element)`.
    ///
    /// This is the in-place version of `map`.
    ///
    /// The order in which elements are visited is unspecified and may not
    /// correspond to the logical order.
    pub fn apply<F: Fn(&T) -> T>(&mut self, f: F) {
        // TODO: Skip unused elements when tensor is not contiguous.
        for val in self.data.as_mut().iter_mut() {
            *val = f(val);
        }
    }

    /// Return a new mutable slice of this tensor.
    ///
    /// Slices are specified in the same way as for [TensorBase::slice].
    pub fn slice_mut<const N: usize, R: IntoSliceItems<N>>(
        &mut self,
        range: R,
    ) -> TensorViewMut<T> {
        self.slice_mut_dyn(&range.into_slice_items())
    }

    /// Return a new mutable slice of this tensor.
    ///
    /// Slices are specified in the same way as for [TensorBase::slice_dyn].
    pub fn slice_mut_dyn(&mut self, range: &[SliceItem]) -> TensorViewMut<T> {
        let (offset_range, layout) = self.layout.slice(range);
        let data = &mut self.data.as_mut()[offset_range];
        TensorViewMut {
            data,
            layout,
            element_type: PhantomData,
        }
    }

    /// Return a mutable view of this tensor.
    ///
    /// Views share the same element array, but can have an independent layout,
    /// with some limitations.
    pub fn view_mut(&mut self) -> TensorViewMut<T> {
        TensorViewMut::new(self.data.as_mut(), &self.layout)
    }

    /// Return an N-dimensional slice of this tensor.
    ///
    /// This is the same as [TensorBase::nd_slice] except that the
    /// returned view can be used to modify elements.
    #[doc(hidden)]
    pub fn nd_slice_mut<const B: usize, const N: usize>(
        &mut self,
        base: [usize; B],
    ) -> NdTensorViewMut<T, N> {
        assert!(B + N == self.ndim());
        let offset = self.layout.slice_offset(base);
        let strides = self.layout.strides()[self.ndim() - N..].try_into().unwrap();
        let shape = self.layout.shape()[self.ndim() - N..].try_into().unwrap();
        let data = &mut self.data_mut()[offset..];
        NdTensorViewMut::from_data(data, shape, Some(strides)).unwrap()
    }

    /// Return a mutable N-dimensional view of this tensor.
    ///
    /// See notes in `[TensorBase::nd_view]`.
    #[doc(hidden)]
    pub fn nd_view_mut<const N: usize>(&mut self) -> NdTensorViewMut<T, N> {
        self.nd_slice_mut([])
    }
}

impl<'a, T> TensorBase<T, &'a mut [T]> {
    /// Consume this view and return the underlying data slice.
    ///
    /// This differs from [Self::data_mut] as the lifetime of the returned slice
    /// is tied to the underlying tensor, rather than the view.
    pub fn into_data_mut(self) -> &'a mut [T] {
        self.data
    }

    /// Return a new view with the dimensions re-ordered according to `dims`.
    pub fn permuted_mut(&mut self, dims: &[usize]) -> TensorViewMut<T> {
        TensorBase {
            data: self.data,
            layout: self.layout.permuted(dims),
            element_type: PhantomData,
        }
    }

    /// Return a new view with the order of dimensions reversed.
    pub fn transposed_mut(&mut self) -> TensorViewMut<T> {
        TensorBase {
            data: self.data,
            layout: self.layout.transposed(),
            element_type: PhantomData,
        }
    }

    /// Return a new view with a given shape. This has the same requirements
    /// as `reshape`.
    pub fn reshaped_mut(&mut self, shape: &[usize]) -> TensorViewMut<T> {
        TensorBase {
            data: self.data,
            layout: self.layout.reshaped(shape),
            element_type: PhantomData,
        }
    }
}

impl<I: TensorIndex, T, S: AsRef<[T]> + AsMut<[T]>> IndexMut<I> for TensorBase<T, S> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let offset = self.layout.offset(index);
        &mut self.data.as_mut()[offset]
    }
}

/// Tensor is an n-dimensional array type, with the number of dimensions
/// determined at runtime.
///
/// Tensor and its associated types [TensorView] and [TensorViewMut] are
/// conceptually a pair of a dynamically sized array (either owned or a
/// reference) and a _layout_ which specifies how to view the contents of that
/// array as an N-dimensional tensor. The layout specifies the number of
/// dimensions, size of each dimension and stride of each dimension (offset
/// between elements in the underlying array).
///
/// Information about a tensor or view's layout is available via the
/// [Layout] trait.
///
/// By default, new tensors have a _contiguous_ layout, in which the stride of
/// the innermost (fastest-changing) dimension `Dn` is 1, the stride of
/// dimension `Di+1` is the size of dimension `Di` and so on. The layout will
/// become non-contiguous if the dimensions are permuted/transposed or if the
/// tensor is sliced in-place. Whether the tensor is contiguous does not matter
/// if accessing elements via indexing, slicing or iterators. It does matter if
/// accessing the underlying element buffer directly.
pub type Tensor<T = f32> = TensorBase<T, Vec<T>>;

impl<T> TensorBase<T, Vec<T>> {
    /// Create a new zero-filled tensor with a given shape.
    pub fn zeros(shape: &[usize]) -> Tensor<T>
    where
        T: Clone + Default,
    {
        let n_elts = shape.iter().product();
        let data = vec![T::default(); n_elts];
        Tensor {
            data,
            layout: DynLayout::new(shape),
            element_type: PhantomData,
        }
    }

    /// Create a new tensor filled with random numbers from a given source.
    pub fn rand<R: RandomSource<T>>(shape: &[usize], rand_src: &mut R) -> Tensor<T>
    where
        T: Clone + Default,
    {
        let mut tensor = Tensor::zeros(shape);
        tensor.data_mut().fill_with(|| rand_src.next());
        tensor
    }

    /// Create a new 0-dimensional (scalar) tensor from a single value.
    pub fn from_scalar(value: T) -> Tensor<T> {
        Self::from_data(&[], vec![value])
    }

    /// Create a new 1-dimensional tensor from a vector. No copying is required.
    pub fn from_vec(data: Vec<T>) -> Tensor<T> {
        Self::from_data(&[data.len()], data)
    }

    /// Clone this tensor with a new shape. The new shape must have the same
    /// total number of elements as the existing shape. See `reshape`.
    pub fn clone_with_shape(&self, shape: &[usize]) -> Tensor<T>
    where
        T: Clone,
    {
        let data = if self.is_contiguous() {
            self.data.clone()
        } else {
            self.view().iter().cloned().collect::<Vec<_>>()
        };
        Self::from_data(shape, data)
    }

    /// Clip dimension `dim` to `[range.start, range.end)`. The new size for
    /// the dimension must be <= the old size.
    ///
    /// This currently requires `T: Copy` to support efficiently moving data
    /// from the new start offset to the beginning of the element buffer.
    pub fn clip_dim(&mut self, dim: usize, range: Range<usize>)
    where
        T: Copy,
    {
        let (start, end) = (range.start, range.end);

        assert!(start <= end, "start must be <= end");
        assert!(end <= self.size(dim), "end must be <= dim size");

        let start_offset = self.layout.stride(dim) * start;
        self.layout.resize_dim(dim, end - start);

        let range = start_offset..start_offset + self.layout.end_offset();
        self.data.copy_within(range.clone(), 0);
        self.data.truncate(range.end - range.start);
    }

    /// Convert the internal layout of elements to be contiguous, as reported
    /// by `is_contiguous`.
    ///
    /// This is a no-op if the tensor is already contiguous.
    pub fn make_contiguous(&mut self)
    where
        T: Clone,
    {
        if self.is_contiguous() {
            return;
        }
        self.data = self.view().iter().cloned().collect();
        self.layout.make_contiguous();
    }

    /// Update the shape of the tensor.
    ///
    /// The total number of elements for the new shape must be the same as the
    /// existing shape.
    ///
    /// This is a cheap operation if the tensor is contiguous, but requires
    /// copying data if the tensor has a non-contiguous layout.
    pub fn reshape(&mut self, shape: &[usize])
    where
        T: Clone,
    {
        let len: usize = shape.iter().product();
        let current_len = self.len();
        assert!(
            len == current_len,
            "New shape must have same total elements as current shape"
        );

        // We currently always copy data whenever the input is non-contiguous.
        // However there are cases of custom strides where copies could be
        // avoided. See https://pytorch.org/docs/stable/generated/torch.Tensor.view.html.
        self.make_contiguous();
        self.layout = DynLayout::new(shape);
    }
}

impl<S: AsRef<[f32]>> TensorBase<f32, S> {
    /// Serialize the tensor to a simple binary format.
    ///
    /// The serialized data is in little-endian order and has the structure:
    ///
    /// ```text
    /// [ndim: u32][dims: u32 * rank][elements: T * product(dims)]
    /// ```
    ///
    /// Where `T` is the tensor's element type.
    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        let ndim: u32 = self.ndim() as u32;
        writer.write_all(&ndim.to_le_bytes())?;
        for &dim in self.shape() {
            writer.write_all(&(dim as u32).to_le_bytes())?;
        }
        for el in self.view().iter() {
            writer.write_all(&el.to_le_bytes())?;
        }
        Ok(())
    }
}

impl<T: PartialEq, S1: AsRef<[T]>, S2: AsRef<[T]>> PartialEq<TensorBase<T, S2>>
    for TensorBase<T, S1>
{
    fn eq(&self, other: &TensorBase<T, S2>) -> bool {
        self.shape() == other.shape() && self.view().iter().eq(other.view().iter())
    }
}

impl<T, S: AsRef<[T]> + Clone> Clone for TensorBase<T, S> {
    fn clone(&self) -> TensorBase<T, S> {
        let data = self.data.clone();
        TensorBase {
            data,
            layout: self.layout.clone(),
            element_type: PhantomData,
        }
    }
}

impl<T, S1: AsRef<[T]>, S2: AsRef<[T]>, const N: usize> From<NdTensorBase<T, S1, N>>
    for TensorBase<T, S2>
where
    S1: Into<S2>,
{
    fn from(value: NdTensorBase<T, S1, N>) -> TensorBase<T, S2> {
        let layout: DynLayout = value.layout().into();
        TensorBase {
            data: value.into_data().into(),
            layout,
            element_type: PhantomData,
        }
    }
}

impl<T> FromIterator<T> for Tensor<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let data: Vec<_> = FromIterator::from_iter(iter);
        Tensor::from_vec(data)
    }
}

impl RandomSource<f32> for XorShiftRng {
    fn next(&mut self) -> f32 {
        self.next_f32()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::IndexMut;

    use crate::rng::XorShiftRng;
    use crate::tensor;
    use crate::{Layout, NdTensor, SliceRange, Tensor, TensorView, TensorViewMut};

    /// Create a tensor where the value of each element is its logical index
    /// plus one.
    fn steps(shape: &[usize]) -> Tensor<i32> {
        let mut x = Tensor::zeros(shape);
        for (index, elt) in x.data_mut().iter_mut().enumerate() {
            *elt = (index + 1) as i32;
        }
        x
    }

    #[test]
    fn test_apply() {
        let mut x = steps(&[3, 3]);
        x.apply(|el| el * el);
        let expected = Tensor::from_data(&[3, 3], vec![1, 4, 9, 16, 25, 36, 49, 64, 81]);
        assert_eq!(x, expected);
    }

    #[test]
    fn test_axis_iter() {
        let x = steps(&[2, 3, 4]);
        let x = x.view();

        // First dimension.
        let views: Vec<_> = x.axis_iter(0).collect();
        assert_eq!(views.len(), 2);
        assert_eq!(views[0], x.slice([0]));
        assert_eq!(views[1], x.slice([1]));

        // Second dimension.
        let views: Vec<_> = x.axis_iter(1).collect();
        assert_eq!(views.len(), 3);
        assert_eq!(views[0], x.slice((.., 0)));
        assert_eq!(views[1], x.slice((.., 1)));
    }

    #[test]
    fn test_axis_iter_mut() {
        let mut x = steps(&[2, 3]);
        let y0 = x.view().slice([0]).to_owned();
        let y1 = x.view().slice([1]).to_owned();

        // First dimension.
        let mut views: Vec<_> = x.axis_iter_mut(0).collect();
        assert_eq!(views.len(), 2);
        assert_eq!(views[0], y0);
        assert_eq!(views[1], y1);
        views[0].iter_mut().for_each(|x| *x += 1);
        views[1].iter_mut().for_each(|x| *x += 2);
        assert_eq!(x.to_vec(), &[2, 3, 4, 6, 7, 8]);

        let z0 = x.view().slice((.., 0)).to_owned();
        let z1 = x.view().slice((.., 1)).to_owned();

        // Second dimension.
        let views: Vec<_> = x.axis_iter_mut(1).collect();
        assert_eq!(views.len(), 3);
        assert_eq!(views[0], z0);
        assert_eq!(views[1], z1);
    }

    #[test]
    fn test_clip_dim() {
        let mut x = steps(&[3, 3]);
        x.clip_dim(0, 1..2);
        x.clip_dim(1, 1..2);
        assert_eq!(x.to_vec(), vec![5]);
    }

    #[test]
    fn test_clip_dim_start() {
        let mut x = steps(&[3, 3]);

        // Clip the start of the tensor, adjusting the `base` offset.
        x.clip_dim(0, 1..3);

        // Indexing should reflect the slice.
        assert_eq!(x.to_vec(), &[4, 5, 6, 7, 8, 9]);
        assert_eq!(x[[0, 0]], 4);
        assert_eq!(*x.index_mut([0, 0]), 4);

        // Slices returned by `data`, `data_mut` should reflect the slice.
        assert_eq!(x.view().data(), &[4, 5, 6, 7, 8, 9]);
        assert_eq!(x.data_mut(), &[4, 5, 6, 7, 8, 9]);

        // Offsets should be relative to the sliced returned by `data`,
        // `data_mut`.
        assert_eq!(x.offsets().collect::<Vec<usize>>(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(x.layout().offset([0, 0]), 0);
    }

    #[test]
    fn test_copy_from() {
        let x = steps(&[3, 3]);
        let mut y = Tensor::zeros(x.shape());

        y.copy_from(&x.view());

        assert_eq!(y, x);
    }

    #[test]
    fn test_from_scalar() {
        let x = Tensor::from_scalar(5);
        assert_eq!(x.shape().len(), 0);
        assert_eq!(x.view().data(), &[5]);
    }

    #[test]
    fn test_from_vec() {
        let x = tensor!([1, 2, 3]);
        assert_eq!(x.shape(), &[3]);
        assert_eq!(x.view().data(), &[1, 2, 3]);
    }

    #[test]
    fn test_from_iterator() {
        let x: Tensor<i32> = FromIterator::from_iter(0..10);
        assert_eq!(x.shape(), &[10]);
        assert_eq!(x.view().data(), &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_stride() {
        let x = Tensor::<f32>::zeros(&[2, 5, 7, 3]);
        assert_eq!(x.stride(3), 1);
        assert_eq!(x.stride(2), 3);
        assert_eq!(x.stride(1), 7 * 3);
        assert_eq!(x.stride(0), 5 * 7 * 3);
    }

    #[test]
    fn test_strides() {
        let x = Tensor::<f32>::zeros(&[2, 5, 7, 3]);
        assert_eq!(x.strides(), [5 * 7 * 3, 7 * 3, 3, 1]);
    }

    #[test]
    fn test_index() {
        let mut x = Tensor::<f32>::zeros(&[2, 2]);

        x.data[0] = 1.0;
        x.data[1] = 2.0;
        x.data[2] = 3.0;
        x.data[3] = 4.0;

        // Index with fixed-sized array.
        assert_eq!(x[[0, 0]], 1.0);
        assert_eq!(x[[0, 1]], 2.0);
        assert_eq!(x[[1, 0]], 3.0);
        assert_eq!(x[[1, 1]], 4.0);

        // Index with slice.
        assert_eq!(x[[0, 0].as_slice()], 1.0);
        assert_eq!(x[[0, 1].as_slice()], 2.0);
        assert_eq!(x[[1, 0].as_slice()], 3.0);
        assert_eq!(x[[1, 1].as_slice()], 4.0);
    }

    #[test]
    fn test_index_scalar() {
        let x = Tensor::from_scalar(5.0);
        assert_eq!(x[[]], 5.0);
    }

    #[test]
    fn test_index_mut() {
        let mut x = Tensor::<f32>::zeros(&[2, 2]);

        x[[0, 0]] = 1.0;
        x[[0, 1]] = 2.0;
        x[[1, 0]] = 3.0;
        x[[1, 1]] = 4.0;

        assert_eq!(x.data[0], 1.0);
        assert_eq!(x.data[1], 2.0);
        assert_eq!(x.data[2], 3.0);
        assert_eq!(x.data[3], 4.0);
    }

    #[test]
    #[should_panic]
    fn test_index_panics_if_invalid() {
        let x = Tensor::<f32>::zeros(&[2, 2]);
        x[[2, 0]];
    }

    #[test]
    #[should_panic]
    fn test_index_panics_if_wrong_dim_count() {
        let x = Tensor::<f32>::zeros(&[2, 2]);
        x[[0, 0, 0]];
    }

    #[test]
    fn test_indices() {
        let x = Tensor::<f32>::zeros(&[2, 2]);
        let x_indices = {
            let mut indices = Vec::new();
            let mut iter = x.indices();
            while let Some(index) = iter.next() {
                indices.push(index.to_vec());
            }
            indices
        };
        assert_eq!(
            x_indices,
            &[vec![0, 0], vec![0, 1], vec![1, 0], vec![1, 1],]
        );
    }

    #[test]
    fn test_item() {
        let scalar = Tensor::from_scalar(5.0);
        assert_eq!(scalar.view().item(), Some(&5.0));

        let vec_one_item = tensor!([5.0]);
        assert_eq!(vec_one_item.view().item(), Some(&5.0));

        let vec_many_items = tensor!([1.0, 2.0]);
        assert_eq!(vec_many_items.view().item(), None);

        let matrix_one_item = Tensor::from_data(&[1, 1], vec![5.0]);
        assert_eq!(matrix_one_item.view().item(), Some(&5.0));
    }

    #[test]
    fn test_map() {
        // Contiguous tensor.
        let x = steps(&[2, 3]).map(|val| val * 2);
        assert_eq!(x.to_vec(), &[2, 4, 6, 8, 10, 12]);

        // Non-contiguous view.
        let x = steps(&[2, 3]);
        let x = x.view().transposed();
        assert!(!x.is_contiguous());
        assert_eq!(x.to_vec(), &[1, 4, 2, 5, 3, 6]);
        let x = x.map(|val| val * 2);
        assert_eq!(x.to_vec(), &[2, 8, 4, 10, 6, 12]);
    }

    #[test]
    fn test_move_axis() {
        let mut x = steps(&[2, 3]);
        x.move_axis(1, 0);
        assert_eq!(x.shape(), [3, 2]);
    }

    #[test]
    fn test_ndim() {
        let scalar = Tensor::from_scalar(5.0);
        let vec = tensor!([5.0]);
        let matrix = Tensor::from_data(&[1, 1], vec![5.0]);

        assert_eq!(scalar.ndim(), 0);
        assert_eq!(vec.ndim(), 1);
        assert_eq!(matrix.ndim(), 2);
    }

    #[test]
    fn test_partial_eq() {
        let x = tensor!([1, 2, 3, 4, 5]);
        let y = x.clone();
        let z = x.clone_with_shape(&[1, 5]);

        // Int tensors are equal if they have the same shape and elements.
        assert_eq!(&x, &y);
        assert_ne!(&x, &z);
    }

    #[test]
    fn test_len() {
        let scalar = Tensor::from_scalar(5);
        let vec = tensor!([1, 2, 3]);
        let matrix = Tensor::from_data(&[2, 2], vec![1, 2, 3, 4]);

        assert_eq!(scalar.len(), 1);
        assert_eq!(vec.len(), 3);
        assert_eq!(matrix.len(), 4);
    }

    #[test]
    fn test_is_empty() {
        assert!(Tensor::<f32>::from_vec(vec![]).is_empty());
        assert!(!tensor!([1]).is_empty());
        assert!(!Tensor::from_scalar(5.0).is_empty());
    }

    #[test]
    fn test_reshape() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 5, 3, 7], &mut rng);
        let x_data: Vec<f32> = x.view().data().into();

        assert_eq!(x.shape(), &[10, 5, 3, 7]);

        x.reshape(&[10, 5, 3 * 7]);

        assert_eq!(x.shape(), &[10, 5, 3 * 7]);
        assert_eq!(x.view().data(), x_data);
    }

    #[test]
    fn test_reshape_non_contiguous() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 10], &mut rng);

        // Set the input up so that it is non-contiguous and has a non-zero
        // `base` offset.
        x.permute(&[1, 0]);
        x.clip_dim(0, 2..8);

        // Reshape the tensor. This should copy the data and reset the `base`
        // offset.
        x.reshape(&[x.shape().iter().product()]);

        // After reshaping, we should be able to successfully read all the elements.
        // Note this test doesn't check that the correct elements were read.
        let elts: Vec<_> = x.view().iter().collect();
        assert_eq!(elts.len(), 60);

        // Set up another input so it is non-contiguous and has a non-zero `base` offset.
        let mut x = steps(&[3, 3]);
        x.clip_dim(0, 1..3);
        x.clip_dim(1, 1..3);

        // Flatten the input with reshape.
        x.reshape(&[4]);

        // Check that the correct elements were read.
        assert_eq!(x.to_vec(), &[5, 6, 8, 9]);
    }

    #[test]
    fn test_reshape_copies_with_custom_strides() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 10], &mut rng);

        // Give the tensor a non-default stride
        x.clip_dim(1, 0..8);
        assert!(!x.is_contiguous());
        let x_elements = x.to_vec();

        x.reshape(&[80]);

        // Since the tensor had a non-default stride, `reshape` will have copied
        // data.
        assert_eq!(x.shape(), &[80]);
        assert!(x.is_contiguous());
        assert_eq!(x.view().data(), x_elements);
    }

    #[test]
    #[should_panic(expected = "New shape must have same total elements as current shape")]
    fn test_reshape_with_wrong_size() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 5, 3, 7], &mut rng);
        x.reshape(&[10, 5]);
    }

    #[test]
    fn test_permute() {
        // Test with a vector (this is a no-op)
        let mut input = steps(&[5]);
        assert!(input.view().iter().eq([1, 2, 3, 4, 5].iter()));
        input.permute(&[0]);
        assert!(input.view().iter().eq([1, 2, 3, 4, 5].iter()));

        // Test with a matrix (ie. transpose the matrix)
        let mut input = steps(&[2, 3]);
        assert!(input.view().iter().eq([1, 2, 3, 4, 5, 6].iter()));
        input.permute(&[1, 0]);
        assert_eq!(input.shape(), &[3, 2]);
        assert!(input.view().iter().eq([1, 4, 2, 5, 3, 6].iter()));

        // Test with a higher-rank tensor. For this test we don't list out the
        // full permuted element sequence, but just check the shape and strides
        // were updated.
        let mut input = steps(&[3, 4, 5]);
        let (stride_0, stride_1, stride_2) = (input.stride(0), input.stride(1), input.stride(2));
        input.permute(&[2, 0, 1]);
        assert_eq!(input.shape(), &[5, 3, 4]);
        assert_eq!(
            (input.stride(0), input.stride(1), input.stride(2)),
            (stride_2, stride_0, stride_1)
        );
    }

    #[test]
    #[should_panic(expected = "permutation is invalid")]
    fn test_permute_wrong_dim_count() {
        let mut input = steps(&[2, 3]);
        input.permute(&[1, 2, 3]);
    }

    #[test]
    fn test_transpose() {
        // Test with a vector (this is a no-op)
        let mut input = steps(&[5]);
        input.transpose();
        assert_eq!(input.shape(), &[5]);

        // Test with a matrix
        let mut input = steps(&[2, 3]);
        assert!(input.view().iter().eq([1, 2, 3, 4, 5, 6].iter()));
        input.transpose();
        assert_eq!(input.shape(), &[3, 2]);
        assert!(input.view().iter().eq([1, 4, 2, 5, 3, 6].iter()));

        // Test with a higher-rank tensor
        let mut input = steps(&[1, 3, 7]);
        input.transpose();
        assert_eq!(input.shape(), [7, 3, 1]);
    }

    #[test]
    fn test_insert_dim() {
        let mut input = steps(&[2, 3]);
        input.insert_dim(1);
        assert_eq!(input.shape(), &[2, 1, 3]);

        input.insert_dim(1);
        assert_eq!(input.shape(), &[2, 1, 1, 3]);

        input.insert_dim(0);
        assert_eq!(input.shape(), &[1, 2, 1, 1, 3]);
    }

    #[test]
    fn test_clone_with_shape() {
        let mut rng = XorShiftRng::new(1234);
        let x = Tensor::rand(&[10, 5, 3, 7], &mut rng);
        let y = x.clone_with_shape(&[10, 5, 3 * 7]);

        assert_eq!(y.shape(), &[10, 5, 3 * 7]);
        assert_eq!(y.view().data(), x.view().data());
    }

    #[test]
    fn test_nd_slice() {
        let mut rng = XorShiftRng::new(1234);
        let x = Tensor::rand(&[10, 5, 3, 7], &mut rng);
        let x_view = x.view().nd_slice([5, 3]);

        for a in 0..x.size(2) {
            for b in 0..x.size(3) {
                assert_eq!(x[[5, 3, a, b]], x_view[[a, b]]);
            }
        }
    }

    #[test]
    fn test_nd_slice_mut() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 5, 3, 7], &mut rng);

        let [_, _, a_size, b_size]: [usize; 4] = x.shape().try_into().unwrap();
        let mut x_view = x.nd_slice_mut([5, 3]);

        for a in 0..a_size {
            for b in 0..b_size {
                x_view[[a, b]] = (a + b) as f32;
            }
        }

        for a in 0..x.size(2) {
            for b in 0..x.size(3) {
                assert_eq!(x[[5, 3, a, b]], (a + b) as f32);
            }
        }
    }

    #[test]
    fn test_iter_for_contiguous_array() {
        for dims in 1..7 {
            let mut shape = Vec::new();
            for d in 0..dims {
                shape.push(d + 1);
            }
            let mut rng = XorShiftRng::new(1234);
            let x = Tensor::rand(&shape, &mut rng);
            let x = x.view();

            let elts: Vec<f32> = x.iter().copied().collect();

            assert_eq!(x.data(), elts);
        }
    }

    #[test]
    fn test_iter_for_empty_array() {
        let empty = Tensor::<f32>::zeros(&[3, 0, 5]);
        assert!(empty.view().iter().next().is_none());
    }

    #[test]
    fn test_iter_for_non_contiguous_array() {
        let mut x = Tensor::zeros(&[3, 3]);
        for (index, elt) in x.data_mut().iter_mut().enumerate() {
            *elt = index + 1;
        }

        // Initially tensor is contiguous, so data buffer and element sequence
        // match.
        let xv = x.view();
        assert_eq!(xv.data(), xv.iter().copied().collect::<Vec<_>>());

        // Slice the tensor along an outer dimension. This will leave the tensor
        // contiguous, and hence `data` and `elements` should return the same
        // elements.
        x.clip_dim(0, 0..2);
        let xv = x.view();
        assert_eq!(xv.data(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(xv.iter().copied().collect::<Vec<_>>(), &[1, 2, 3, 4, 5, 6]);
        // Test with step > 1 to exercise `Elements::nth`.
        assert_eq!(
            xv.iter().step_by(2).copied().collect::<Vec<_>>(),
            &[1, 3, 5]
        );

        // Slice the tensor along an inner dimension. The tensor will no longer
        // be contiguous and hence `elements` will return different results than
        // `data`.
        x.clip_dim(1, 0..2);
        let xv = x.view();
        assert_eq!(xv.data(), &[1, 2, 3, 4, 5]);
        assert_eq!(xv.iter().copied().collect::<Vec<_>>(), &[1, 2, 4, 5]);
        // Test with step > 1 to exercise `Elements::nth`.
        assert_eq!(xv.iter().step_by(2).copied().collect::<Vec<_>>(), &[1, 4]);
    }

    // PyTorch and numpy do not allow iteration over a scalar, but it seems
    // consistent for `Tensor::iter` to always yield `Tensor::len` elements,
    // and `len` returns 1 for a scalar.
    #[test]
    fn test_iter_for_scalar() {
        let x = Tensor::from_scalar(5.0);
        let elements = x.view().iter().copied().collect::<Vec<_>>();
        assert_eq!(&elements, &[5.0]);
    }

    #[test]
    fn test_iter_mut_for_contiguous_array() {
        for dims in 1..7 {
            let mut shape = Vec::new();
            for d in 0..dims {
                shape.push(d + 1);
            }
            let mut rng = XorShiftRng::new(1234);
            let mut x = Tensor::rand(&shape, &mut rng);

            let elts: Vec<f32> = x.view().iter().map(|x| x * 2.).collect();

            for elt in x.iter_mut() {
                *elt *= 2.;
            }

            assert_eq!(x.view().data(), elts);
        }
    }

    #[test]
    fn test_iter_mut_for_non_contiguous_array() {
        let mut x = Tensor::zeros(&[3, 3]);
        for (index, elt) in x.data_mut().iter_mut().enumerate() {
            *elt = index + 1;
        }
        x.permute(&[1, 0]);

        let x_doubled: Vec<usize> = x.view().iter().map(|x| x * 2).collect();
        for elt in x.iter_mut() {
            *elt *= 2;
        }
        assert_eq!(x.to_vec(), x_doubled);
    }

    #[test]
    fn test_to_vec() {
        let mut x = steps(&[3, 3]);

        // Contiguous case. This should use the fast-path.
        assert_eq!(x.to_vec(), x.view().iter().copied().collect::<Vec<_>>());

        // Non-contiguous case.
        x.clip_dim(1, 0..2);
        assert!(!x.is_contiguous());
        assert_eq!(x.to_vec(), x.view().iter().copied().collect::<Vec<_>>());
    }

    #[test]
    fn test_offsets() {
        let mut rng = XorShiftRng::new(1234);
        let mut x = Tensor::rand(&[10, 10], &mut rng);

        let x_elts: Vec<_> = x.to_vec();

        let x_offsets = x.offsets();
        let x_data = x.data_mut();
        let x_elts_from_offset: Vec<_> = x_offsets.map(|off| x_data[off]).collect();

        assert_eq!(x_elts, x_elts_from_offset);
    }

    #[test]
    fn test_offsets_nth() {
        let x = steps(&[3]);
        let mut iter = x.offsets();
        assert_eq!(iter.nth(0), Some(0));
        assert_eq!(iter.nth(0), Some(1));
        assert_eq!(iter.nth(0), Some(2));
        assert_eq!(iter.nth(0), None);

        let x = steps(&[10]);
        let mut iter = x.offsets();
        assert_eq!(iter.nth(1), Some(1));
        assert_eq!(iter.nth(5), Some(7));
        assert_eq!(iter.nth(1), Some(9));
        assert_eq!(iter.nth(0), None);
    }

    #[test]
    fn test_from_data() {
        let scalar = Tensor::from_data(&[], vec![1.0]);
        assert_eq!(scalar.len(), 1);

        let matrix = Tensor::from_data(&[2, 2], vec![1, 2, 3, 4]);
        assert_eq!(matrix.shape(), &[2, 2]);
        assert_eq!(matrix.view().data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_from_data_with_slice() {
        let matrix = TensorView::from_data(&[2, 2], [1, 2, 3, 4].as_slice());
        assert_eq!(matrix.shape(), &[2, 2]);
        assert_eq!(matrix.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_from_data_with_mut_slice() {
        let mut data = vec![1, 2, 3, 4];
        let mut matrix = TensorViewMut::from_data(&[2, 2], &mut data[..]);
        matrix[[0, 1]] = 5;
        matrix[[1, 0]] = 6;
        assert_eq!(data, &[1, 5, 6, 4]);
    }

    #[test]
    #[should_panic]
    fn test_from_data_panics_with_wrong_len() {
        Tensor::from_data(&[1], vec![1, 2, 3]);
    }

    #[test]
    #[should_panic]
    fn test_from_data_panics_if_scalar_data_empty() {
        Tensor::<i32>::from_data(&[], vec![]);
    }

    #[test]
    #[should_panic]
    fn test_from_data_panics_if_scalar_data_has_many_elements() {
        Tensor::from_data(&[], vec![1, 2, 3]);
    }

    #[test]
    fn test_from_ndtensor() {
        // NdTensor -> Tensor
        let ndtensor = NdTensor::zeros([1, 10, 20]);
        let tensor: Tensor<i32> = ndtensor.clone().into();
        assert_eq!(tensor.view().data(), ndtensor.view().data());
        assert_eq!(tensor.shape(), ndtensor.shape());
        assert_eq!(tensor.strides(), ndtensor.strides());

        // NdTensorView -> TensorView
        let view: TensorView<i32> = ndtensor.view().into();
        assert_eq!(view.shape(), ndtensor.shape());

        // NdTensorViewMut -> TensorViewMut
        let mut ndtensor = NdTensor::zeros([1, 10, 20]);
        let mut view: TensorViewMut<i32> = ndtensor.view_mut().into();
        view[[0, 0, 0]] = 1;
        assert_eq!(ndtensor[[0, 0, 0]], 1);
    }

    #[test]
    fn test_is_contiguous() {
        let mut x = Tensor::zeros(&[3, 3]);
        for (index, elt) in x.data_mut().iter_mut().enumerate() {
            *elt = index + 1;
        }

        // Freshly-allocated tensor
        assert!(x.is_contiguous());

        // Tensor where outermost dimension has been clipped at the end.
        let mut y = x.clone();
        y.clip_dim(0, 0..2);
        assert!(y.is_contiguous());
        assert_eq!(y.view().data(), &[1, 2, 3, 4, 5, 6]);

        // Tensor where outermost dimension has been clipped at the start.
        let mut y = x.clone();
        y.clip_dim(0, 1..3);
        assert!(y.is_contiguous());
        assert_eq!(y.view().data(), &[4, 5, 6, 7, 8, 9]);

        // Tensor where inner dimension has been clipped at the start.
        let mut y = x.clone();
        y.clip_dim(1, 1..3);
        assert!(!y.is_contiguous());

        // Tensor where inner dimension has been clipped at the end.
        let mut y = x.clone();
        y.clip_dim(1, 0..2);
        assert!(!y.is_contiguous());
    }

    #[test]
    fn test_is_contiguous_1d() {
        let mut x = Tensor::zeros(&[10]);
        for (index, elt) in x.data_mut().iter_mut().enumerate() {
            *elt = index + 1;
        }

        assert!(x.is_contiguous());
        x.clip_dim(0, 0..5);
        assert!(x.is_contiguous());
    }

    #[test]
    fn test_make_contiguous() {
        let mut x = steps(&[3, 3]);
        assert!(x.is_contiguous());

        // Clip outer dimension at start. This will modify the base offset.
        x.clip_dim(0, 1..3);

        // Clip inner dimension at start. This will modify the strides.
        x.clip_dim(1, 1..3);
        assert!(!x.is_contiguous());

        x.make_contiguous();
        assert!(x.is_contiguous());
        assert_eq!(x.to_vec(), &[5, 6, 8, 9]);
    }

    #[test]
    fn test_to_contiguous() {
        let x = steps(&[3, 3]);
        let x = x.view();
        let y = x.to_contiguous();
        let y = y.view();
        assert!(y.is_contiguous());
        assert_eq!(y.data().as_ptr(), x.data().as_ptr());

        let x = x.permuted(&[1, 0]);
        let y = x.to_contiguous();
        let y = y.view();
        assert!(y.is_contiguous());
        assert_ne!(y.data().as_ptr(), x.data().as_ptr());
        assert_eq!(y.data(), x.to_vec());
    }

    #[test]
    fn test_broadcast_iter() {
        let x = steps(&[1, 2, 1, 2]);
        let x = x.view();
        assert_eq!(x.to_vec(), &[1, 2, 3, 4]);

        // Broadcast a 1-size dimension to size 2
        let bx = x.broadcast_iter(&[2, 2, 1, 2]);
        assert_eq!(bx.copied().collect::<Vec<i32>>(), &[1, 2, 3, 4, 1, 2, 3, 4]);

        // Broadcast a different 1-size dimension to size 2
        let bx = x.broadcast_iter(&[1, 2, 2, 2]);
        assert_eq!(bx.copied().collect::<Vec<i32>>(), &[1, 2, 1, 2, 3, 4, 3, 4]);

        // Broadcast to a larger number of dimensions
        let x = steps(&[5]);
        let bx = x.view().broadcast_iter(&[1, 5]);
        assert_eq!(bx.copied().collect::<Vec<i32>>(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_broadcast_iter_with_scalar() {
        let scalar = Tensor::from_scalar(7);
        let bx = scalar.view().broadcast_iter(&[3, 3]);
        assert_eq!(
            bx.copied().collect::<Vec<i32>>(),
            &[7, 7, 7, 7, 7, 7, 7, 7, 7]
        );
    }

    #[test]
    #[should_panic(expected = "Cannot broadcast to specified shape")]
    fn test_broadcast_iter_with_invalid_shape() {
        let x = steps(&[2, 2]);
        x.view().broadcast_iter(&[3, 2]);
    }

    #[test]
    #[should_panic(expected = "Cannot broadcast to specified shape")]
    fn test_broadcast_iter_with_shorter_shape() {
        let x = steps(&[2, 2]);
        x.view().broadcast_iter(&[4]);
    }

    #[test]
    fn test_broadcast_offsets() {
        let x = steps(&[2, 1, 4]);
        let x = x.view();
        let to_shape = &[2, 2, 1, 4];

        let expected: Vec<i32> = x.broadcast_iter(to_shape).copied().collect();
        let actual: Vec<i32> = x
            .broadcast_offsets(to_shape)
            .map(|off| x.data()[off])
            .collect();

        assert_eq!(&actual, &expected);
    }

    #[test]
    #[should_panic(expected = "Cannot broadcast to specified shape")]
    fn test_broadcast_offsets_with_invalid_shape() {
        let x = steps(&[2, 2]);
        x.broadcast_offsets(&[3, 2]);
    }

    #[test]
    fn test_can_broadcast_to() {
        let x = steps(&[1, 5, 10]);
        assert!(x.can_broadcast_to(&[2, 5, 10]));
        assert!(x.can_broadcast_to(&[1, 5, 10]));
        assert!(!x.can_broadcast_to(&[1, 1, 10]));
    }

    #[test]
    fn test_can_broadcast_with() {
        let x = steps(&[1, 5, 10]);
        assert!(x.can_broadcast_with(&[2, 5, 10]));
        assert!(x.can_broadcast_with(&[1, 5, 10]));
        assert!(x.can_broadcast_with(&[1, 1, 10]));
    }

    // Common slice tests for all slicing functions.
    macro_rules! slice_tests {
        ($x:expr, $method:ident) => {
            assert_eq!($x.shape(), &[2, 3, 4]);

            // 1D index
            let y = $x.$method([0]);
            assert_eq!(y.shape(), [3, 4]);
            assert_eq!(y.to_vec(), (1..=(3 * 4)).into_iter().collect::<Vec<i32>>());

            // 2D index
            let y = $x.$method([0, 1]);
            assert_eq!(y.shape(), [4]);
            assert_eq!(y.to_vec(), (5..=8).into_iter().collect::<Vec<i32>>());

            // 3D index
            let y = $x.$method([0, 1, 2]);
            assert_eq!(y.shape(), []);
            assert_eq!(y.view().item(), Some(&7));

            // Full range
            let y = $x.$method([..]);
            assert_eq!(y.shape(), [2, 3, 4]);
            assert_eq!(y.to_vec(), $x.to_vec());

            // Partial ranges
            let y = $x.$method((.., ..2, 1..));
            assert_eq!(y.shape(), [2, 2, 3]);

            // Mixed indices and ranges
            let y = $x.$method((.., 0, ..));
            assert_eq!(y.shape(), [2, 4]);

            let y = $x.$method((.., .., 0));
            assert_eq!(y.shape(), [2, 3]);
            assert_eq!(y.to_vec(), &[1, 5, 9, 13, 17, 21]);
        };
    }

    #[test]
    fn test_slice() {
        let x = steps(&[2, 3, 4]);
        slice_tests!(x.view(), slice);
    }

    #[test]
    fn test_slice_mut() {
        let mut x = steps(&[2, 3, 4]);
        slice_tests!(x, slice_mut);
    }

    #[test]
    fn test_slice_iter() {
        let sr = |start, end| SliceRange::new(start, end, 1);
        let x = steps(&[3, 3]);
        let x = x.view();

        // Slice that removes start of each dimension
        let slice: Vec<_> = x.slice_iter(&[sr(1, 3), sr(1, 3)]).copied().collect();
        assert_eq!(slice, &[5, 6, 8, 9]);

        // Slice that removes end of each dimension
        let slice: Vec<_> = x.slice_iter(&[sr(0, 2), sr(0, 2)]).copied().collect();
        assert_eq!(slice, &[1, 2, 4, 5]);

        // Slice that removes start and end of first dimension
        let slice: Vec<_> = x.slice_iter(&[sr(1, 2), sr(0, 3)]).copied().collect();
        assert_eq!(slice, &[4, 5, 6]);

        // Slice that removes start and end of second dimension
        let slice: Vec<_> = x.slice_iter(&[sr(0, 3), sr(1, 2)]).copied().collect();
        assert_eq!(slice, &[2, 5, 8]);
    }

    #[test]
    fn test_slice_iter_with_step() {
        let sr = SliceRange::new;
        let x = steps(&[10]);
        let x = x.view();

        // Positive steps > 1.
        let slice: Vec<_> = x.slice_iter(&[sr(0, 10, 2)]).copied().collect();
        assert_eq!(slice, &[1, 3, 5, 7, 9]);

        let slice: Vec<_> = x.slice_iter(&[sr(0, 10, 3)]).copied().collect();
        assert_eq!(slice, &[1, 4, 7, 10]);

        let slice: Vec<_> = x.slice_iter(&[sr(0, 10, 10)]).copied().collect();
        assert_eq!(slice, &[1]);

        // Negative steps.
        let slice: Vec<_> = x.slice_iter(&[sr(10, -11, -1)]).copied().collect();
        assert_eq!(slice, &[10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);

        let slice: Vec<_> = x.slice_iter(&[sr(8, 0, -1)]).copied().collect();
        assert_eq!(slice, &[9, 8, 7, 6, 5, 4, 3, 2]);

        let slice: Vec<_> = x.slice_iter(&[sr(10, 0, -2)]).copied().collect();
        assert_eq!(slice, &[10, 8, 6, 4, 2]);

        let slice: Vec<_> = x.slice_iter(&[sr(10, 0, -10)]).copied().collect();
        assert_eq!(slice, &[10]);
    }

    #[test]
    fn test_slice_iter_negative_indices() {
        let sr = |start, end| SliceRange::new(start, end, 1);
        let x = steps(&[10]);
        let x = x.view();

        // Negative start
        let slice: Vec<_> = x.slice_iter(&[sr(-2, 10)]).copied().collect();
        assert_eq!(slice, &[9, 10]);

        // Negative end
        let slice: Vec<_> = x.slice_iter(&[sr(7, -1)]).copied().collect();
        assert_eq!(slice, &[8, 9]);

        // Negative start and end
        let slice: Vec<_> = x.slice_iter(&[sr(-3, -1)]).copied().collect();
        assert_eq!(slice, &[8, 9]);
    }

    #[test]
    fn test_slice_iter_clamps_indices() {
        let sr = SliceRange::new;
        let x = steps(&[5]);
        let x = x.view();

        // Test cases for positive steps (ie. traversing forwards).

        // Positive start out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(10, 11, 1)]).collect();
        assert_eq!(slice.len(), 0);

        // Positive end out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(0, 10, 1)]).copied().collect();
        assert_eq!(slice, &[1, 2, 3, 4, 5]);

        // Negative start out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(-10, 5, 1)]).copied().collect();
        assert_eq!(slice, &[1, 2, 3, 4, 5]);

        // Negative end out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(-10, -5, 1)]).collect();
        assert_eq!(slice.len(), 0);

        // Test cases for negative steps (ie. traversing backwards).

        // Positive start out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(10, -6, -1)]).copied().collect();
        assert_eq!(slice, &[5, 4, 3, 2, 1]);

        // Positive end out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(0, 10, -1)]).collect();
        assert_eq!(slice.len(), 0);

        // Negative start out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(-10, 5, -1)]).collect();
        assert_eq!(slice.len(), 0);

        // Negative end out of bounds
        let slice: Vec<_> = x.slice_iter(&[sr(-1, -10, -1)]).copied().collect();
        assert_eq!(slice, &[5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_slice_iter_start_end_step_combinations() {
        let sr = SliceRange::new;
        let x = steps(&[3]);
        let x = x.view();

        // Test various combinations of slice starts, ends and steps that are
        // positive and negative, in-bounds and out-of-bounds, and ensure they
        // don't cause a panic.
        for start in -5..5 {
            for end in -5..5 {
                for step in -5..5 {
                    if step == 0 {
                        continue;
                    }
                    x.slice_iter(&[sr(start, end, step)]).for_each(drop);
                }
            }
        }
    }

    // These tests assume the correctness of `slice_iter`, given the tests
    // above, and check for consistency between the results of `slice_offsets`
    // and `slice_iter`.
    #[test]
    fn test_slice_offsets() {
        let x = steps(&[5, 5]);
        let x = x.view();

        // Range that removes the start and end of each dimension.
        let range = &[SliceRange::new(1, 4, 1), SliceRange::new(1, 4, 1)];
        let expected: Vec<_> = x.slice_iter(range).copied().collect();
        let result: Vec<_> = x
            .slice_offsets(range)
            .map(|offset| x.data()[offset])
            .collect();

        assert_eq!(&result, &expected);
    }

    #[test]
    fn test_squeezed() {
        let mut rng = XorShiftRng::new(1234);
        let x = Tensor::rand(&[1, 1, 10, 20], &mut rng);
        let x = x.view();
        let y = x.squeezed();
        assert_eq!(y.data(), x.data());
        assert_eq!(y.shape(), &[10, 20]);
        assert_eq!(y.stride(0), 20);
        assert_eq!(y.stride(1), 1);
    }

    #[test]
    fn test_write() -> std::io::Result<()> {
        use std::io::{Cursor, Read};
        let x = Tensor::from_data(&[2, 3], vec![1., 2., 3., 4., 5., 6.]);
        let x = x.view();
        let mut buf: Vec<u8> = Vec::new();

        x.write(&mut buf)?;

        assert_eq!(buf.len(), 4 + x.ndim() * 4 + x.len() * 4);

        let mut cursor = Cursor::new(buf);
        let mut tmp = [0u8; 4];

        cursor.read(&mut tmp)?;
        let ndim = u32::from_le_bytes(tmp);
        assert_eq!(ndim, x.ndim() as u32);

        for &size in x.shape().iter() {
            cursor.read(&mut tmp)?;
            let written_size = u32::from_le_bytes(tmp);
            assert_eq!(written_size, size as u32);
        }

        for el in x.iter().copied() {
            cursor.read(&mut tmp)?;
            let written_el = f32::from_le_bytes(tmp);
            assert_eq!(written_el, el);
        }

        Ok(())
    }
}
