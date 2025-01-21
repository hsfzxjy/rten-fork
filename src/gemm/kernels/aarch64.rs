use std::arch::aarch64::{float32x4_t, int32x4_t};
use std::mem::MaybeUninit;
use std::ops::Range;

use rten_simd::vec_count;
use rten_tensor::{Matrix, MatrixLayout};

use super::simd_generic::{simd_gemv, simd_int8_gemm, simd_int8_gemv, GemmDispatch};
use super::{extract_zero_points, Kernel, Lhs, PackedLayout, QuantParams, TempTile};
use crate::gemm::packing::{pack_a_block, pack_b_block, packed_a_layout, packed_b_layout};
use crate::gemm::{packing, Im2Col};
use crate::slice_cast::{cast_pod_mut_slice, cast_pod_slice};

pub struct ArmNeonKernel {
    _private: (),
}

impl ArmNeonKernel {
    const MR: usize = 8;
    const NR: usize = 8;
}

// Safety - We assume that Rust code on Arm is always compiled with Arm Neon
// available.
unsafe impl Kernel<f32, f32, f32> for ArmNeonKernel {
    fn new() -> Option<Self> {
        Some(ArmNeonKernel { _private: () })
    }

    fn name(&self) -> &'static str {
        "arm-neon"
    }

    fn mr(&self) -> usize {
        Self::MR
    }

    fn nr(&self) -> usize {
        Self::NR
    }

    fn packed_a_layout(
        &self,
        a: Matrix,
        rows: usize,
        cols: usize,
        _quant: Option<QuantParams<f32>>,
    ) -> PackedLayout {
        let mut info = packed_a_layout::<f32, { Self::MR }>(rows, cols);
        info.must_pack = a.col_stride() != 1;
        info
    }

    fn pack_a_block(
        &self,
        out: &mut [MaybeUninit<u8>],
        a: Matrix,
        rows: Range<usize>,
        cols: Range<usize>,
        _quant: Option<QuantParams<f32>>,
    ) {
        let out = cast_pod_mut_slice(out).expect("incorrect alignment for packing buffer");
        pack_a_block::<f32, { Self::MR }>(out, a, rows, cols);
    }

    fn packed_b_layout(
        &self,
        rows: usize,
        cols: usize,
        _quant: Option<QuantParams<f32>>,
    ) -> PackedLayout {
        packed_b_layout::<f32, { Self::NR }>(rows, cols)
    }

    fn pack_b_block(
        &self,
        out: &mut [MaybeUninit<u8>],
        b: Matrix,
        rows: Range<usize>,
        cols: Range<usize>,
        _quant: Option<QuantParams<f32>>,
    ) {
        let out = cast_pod_mut_slice(out).expect("incorrect alignment for packing buffer");
        pack_b_block::<f32, { Self::NR }>(out, b, rows, cols);
    }

    fn pack_im2col(
        &self,
        out: &mut [MaybeUninit<u8>],
        image: &Im2Col<f32>,
        rows: Range<usize>,
        cols: Range<usize>,
    ) {
        const NR_REGS: usize = vec_count::<float32x4_t>(ArmNeonKernel::NR);

        // Safety: Arm Neon instructions are supported
        let out = cast_pod_mut_slice(out).unwrap();
        unsafe {
            image.pack_block::<int32x4_t, NR_REGS>(out, Self::NR, rows, cols);
        }
    }

    unsafe fn kernel(
        &self,
        tile_ptr: *mut f32,
        tile_row_stride: usize,
        a: Lhs<f32>,
        b: &[u8],
        used_rows: usize,
        used_cols: usize,
        depth: usize,
        alpha: f32,
        beta: f32,
        _a_quant: Option<QuantParams<f32>>,
        _b_quant: Option<QuantParams<f32>>,
    ) {
        const MR: usize = ArmNeonKernel::MR;
        const NR: usize = ArmNeonKernel::NR;
        const NR_REGS: usize = vec_count::<float32x4_t>(NR);

        let b = cast_pod_slice(b).unwrap();

        let mut tmp_tile = TempTile::<f32, MR, NR>::new();
        let (dest_ptr, dest_row_stride, dest_beta) = if used_cols == NR {
            (tile_ptr, tile_row_stride, beta)
        } else {
            (tmp_tile.as_mut_ptr() as *mut f32, NR, 0.)
        };

        let gemm = GemmDispatch::<float32x4_t, MR, NR_REGS>::new(
            dest_ptr,
            dest_row_stride,
            a,
            b,
            depth,
            alpha,
            dest_beta,
        );

        match used_rows {
            8 => gemm.dispatch::<8>(),
            7 => gemm.dispatch::<7>(),
            6 => gemm.dispatch::<6>(),
            5 => gemm.dispatch::<5>(),
            4 => gemm.dispatch::<4>(),
            3 => gemm.dispatch::<3>(),
            2 => gemm.dispatch::<2>(),
            1 => gemm.dispatch::<1>(),
            _ => panic!("unsupported `used_rows` {}", used_rows),
        }

        if used_cols != NR {
            tmp_tile.accumulate_into(
                tile_ptr as *mut MaybeUninit<f32>,
                used_rows,
                used_cols,
                tile_row_stride,
                beta,
            );
        }
    }

    fn gemv_kernel(
        &self,
        out: &mut [MaybeUninit<f32>],
        a: &[f32],
        b: Matrix,
        alpha: f32,
        beta: f32,
        _a_quant: Option<QuantParams<f32>>,
        _b_quant: Option<QuantParams<f32>>,
    ) {
        // Safety - float32x4_t is supported if this kernel was constructed.
        unsafe {
            simd_gemv::<float32x4_t, 4>(out, a, b, alpha, beta);
        }
    }
}

/// 8-bit integer matrix multiplication kernel using Arm dot product
/// instructions.
pub struct ArmInt8DotKernel {
    _private: (),
}

impl ArmInt8DotKernel {
    const MR: usize = 8;
    const NR: usize = 4;
}

unsafe impl Kernel<u8, i8, i32> for ArmInt8DotKernel {
    fn new() -> Option<Self> {
        // This kernel requires i8mm for the USDOT instruction. This is
        // convenient as it lets us use the same kernel as x64 but requires
        // Arm v8.6+ (also optionally included in some earlier versions).
        //
        // With some modifications it should be possible to use UDOT instead
        // which is much more widely available.
        if !std::arch::is_aarch64_feature_detected!("i8mm") {
            return None;
        }
        Some(ArmInt8DotKernel { _private: () })
    }

    fn name(&self) -> &'static str {
        "arm-udot-int8"
    }

    fn mr(&self) -> usize {
        Self::MR
    }

    fn nr(&self) -> usize {
        Self::NR
    }

    fn packed_a_layout(
        &self,
        _a: Matrix<u8>,
        rows: usize,
        cols: usize,
        _quant: Option<QuantParams<u8>>,
    ) -> PackedLayout {
        let mut layout = packing::int8::packed_a_layout::<{ Self::MR }>(rows, cols);
        layout.must_pack = true;
        layout
    }

    fn pack_a_block(
        &self,
        out: &mut [MaybeUninit<u8>],
        a: Matrix<u8>,
        rows: Range<usize>,
        cols: Range<usize>,
        _quant: Option<QuantParams<u8>>,
    ) {
        let out = cast_pod_mut_slice(out).unwrap();
        packing::int8::pack_a::<{ Self::MR }>(out, a.slice((rows, cols)))
    }

    fn packed_b_layout(
        &self,
        rows: usize,
        cols: usize,
        _quant: Option<QuantParams<i8>>,
    ) -> PackedLayout {
        packing::int8::packed_b_layout::<{ Self::NR }>(rows, cols)
    }

    fn pack_b_block(
        &self,
        out: &mut [MaybeUninit<u8>],
        b: Matrix<i8>,
        rows: Range<usize>,
        cols: Range<usize>,
        _quant: Option<QuantParams<i8>>,
    ) {
        let out = cast_pod_mut_slice(out).unwrap();
        packing::int8::pack_b::<{ Self::NR }>(out, b.slice((rows, cols)))
    }

    fn pack_im2col(
        &self,
        _out: &mut [MaybeUninit<u8>],
        _image: &Im2Col<i8>,
        _rows: Range<usize>,
        _cols: Range<usize>,
    ) {
        unimplemented!("pack_im2col not implemented");
    }

    #[target_feature(enable = "i8mm")]
    unsafe fn kernel(
        &self,
        tile_ptr: *mut i32,
        tile_row_stride: usize,
        a: Lhs<u8>,
        b: &[u8],
        used_rows: usize,
        used_cols: usize,
        depth: usize,
        _alpha: f32,
        beta: i32,
        a_quant: Option<QuantParams<u8>>,
        b_quant: Option<QuantParams<i8>>,
    ) {
        let a_data = match a {
            Lhs::Packed(data) => data,
            Lhs::Unpacked { .. } => panic!("lhs must be packed"),
        };

        let a_zero_points = extract_zero_points(a_quant, used_rows);
        let b_zero_points = extract_zero_points(b_quant, used_cols);
        let (a_data, a_row_sums) = packing::int8::extract_packed_a::<{ Self::MR }>(a_data);
        let (b, b_col_sums) = packing::int8::extract_packed_b::<{ Self::NR }>(b);

        simd_int8_gemm::<_, { Self::MR }, { Self::NR }>(
            tile_ptr,
            tile_row_stride,
            a_data,
            b,
            used_rows,
            used_cols,
            depth,
            beta != 0, // accumulate
            a_zero_points,
            b_zero_points,
            a_row_sums,
            b_col_sums,
            usdot,
        )
    }

    fn gemv_kernel(
        &self,
        out: &mut [MaybeUninit<i32>],
        a: &[u8],
        b: Matrix<i8>,
        _alpha: f32,
        beta: i32,
        a_quant: Option<QuantParams<u8>>,
        b_quant: Option<QuantParams<i8>>,
    ) {
        let a_zero = a_quant.map(|aq| aq.zero_point[0]).unwrap_or(0);
        let b_zero = b_quant.map(|bq| bq.zero_point);
        let accumulate = beta != 0;

        #[target_feature(enable = "i8mm")]
        unsafe fn gemv_impl(
            out: &mut [MaybeUninit<i32>],
            a: &[u8],
            b: Matrix<i8>,
            accumulate: bool,
            a_zero: u8,
            b_zero: Option<&[i8]>,
        ) {
            simd_int8_gemv(out, a, b, accumulate, a_zero, b_zero, usdot)
        }

        // Safety: We checked target feature support when kernel was
        // constructed.
        unsafe { gemv_impl(out, a, b, accumulate, a_zero, b_zero) }
    }
}

/// Compute dot product of groups of 4 u8 ints with 4 i8 ints and accumulate
/// into 4 i32 values.
///
/// `a` and `b` are intepreted as `uint8x16_t` and ``int8x16_t` respectively.
#[target_feature(enable = "i8mm")]
#[inline]
unsafe fn usdot(a: int32x4_t, b: int32x4_t, c: int32x4_t) -> int32x4_t {
    use core::arch::aarch64::{
        vreinterpretq_s32_u32, vreinterpretq_s8_s32, vreinterpretq_u32_s32, vreinterpretq_u8_s32,
    };
    let a = vreinterpretq_u8_s32(a);
    let b = vreinterpretq_s8_s32(b);
    let mut c = vreinterpretq_u32_s32(c);

    // Use inline asm here because the `vusdotq_s32` intrinsic is not
    // stabilized yet.
    use std::arch::asm;
    asm! {
        "usdot {result:v}.4s, {a:v}.16b, {b:v}.16b",
        result = inout(vreg) c,
        a = in(vreg) a,
        b = in(vreg) b,
        options(nostack)
    }
    vreinterpretq_s32_u32(c)
}
