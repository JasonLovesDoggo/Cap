// RGB to limited-range BT.709 Y'CbCr in 16.16 fixed point, rounded to the
// nearest code value: the matrix and range every export is tagged with. The
// luma row sums to 219/255 and each chroma row to 0, so the grey axis is
// exact (black 16/128/128, white 235/128/128).
const BT709_Y: [i32; 3] = [11966, 40254, 4064];
const BT709_CB: [i32; 3] = [-6596, -22189, 28785];
const BT709_CR: [i32; 3] = [28785, -26145, -2640];

#[inline(always)]
fn fixed_16(row: [i32; 3], r: i32, g: i32, b: i32) -> i32 {
    (row[0] * r + row[1] * g + row[2] * b + (1 << 15)) >> 16
}

/// BT.709 limited-range luma of an 8-bit RGB pixel.
#[inline(always)]
pub(crate) fn bt709_luma(r: i32, g: i32, b: i32) -> u8 {
    (16 + fixed_16(BT709_Y, r, g, b)).clamp(16, 235) as u8
}

/// BT.709 limited-range (Cb, Cr) of an 8-bit RGB pixel (for 4:2:0, the mean
/// of the 2x2 block).
#[inline(always)]
pub(crate) fn bt709_chroma(r: i32, g: i32, b: i32) -> [u8; 2] {
    [
        (128 + fixed_16(BT709_CB, r, g, b)).clamp(16, 240) as u8,
        (128 + fixed_16(BT709_CR, r, g, b)).clamp(16, 240) as u8,
    ]
}

pub fn rgba_to_nv12_bt709(
    rgba: &[u8],
    width: u32,
    height: u32,
    rgba_stride: u32,
    output: &mut [u8],
) {
    let width = width as usize;
    let height = height as usize;
    let stride = rgba_stride as usize;
    if width == 0 || height == 0 {
        return;
    }

    let row_bytes = width.checked_mul(4).expect("RGBA row size overflow");
    let input_bytes = (height - 1)
        .checked_mul(stride)
        .and_then(|bytes| bytes.checked_add(row_bytes))
        .expect("RGBA frame size overflow");
    let y_bytes = width.checked_mul(height).expect("NV12 luma size overflow");
    let output_bytes = width
        .checked_mul(height / 2)
        .and_then(|bytes| bytes.checked_add(y_bytes))
        .expect("NV12 frame size overflow");
    assert!(
        stride >= row_bytes,
        "RGBA stride is shorter than a pixel row"
    );
    assert!(rgba.len() >= input_bytes, "RGBA frame is truncated");
    assert!(output.len() >= output_bytes, "NV12 output is truncated");

    scalar(rgba, width, height, stride, &mut output[..output_bytes]);
}

fn scalar(rgba: &[u8], width: usize, height: usize, stride: usize, output: &mut [u8]) {
    let (y_plane, uv_plane) = output.split_at_mut(width * height);
    for row in 0..height {
        let src_row = &rgba[row * stride..row * stride + width * 4];
        let y_row = &mut y_plane[row * width..(row + 1) * width];
        for col in 0..width {
            y_row[col] = bt709_luma(
                src_row[col * 4] as i32,
                src_row[col * 4 + 1] as i32,
                src_row[col * 4 + 2] as i32,
            );
        }
    }
    for row in 0..height / 2 {
        let top = &rgba[row * 2 * stride..row * 2 * stride + width * 4];
        let bottom = &rgba[(row * 2 + 1) * stride..(row * 2 + 1) * stride + width * 4];
        let uv = &mut uv_plane[row * width..(row + 1) * width];
        for col in 0..width / 2 {
            let avg = |channel| {
                (top[col * 8 + channel] as i32
                    + top[col * 8 + channel + 4] as i32
                    + bottom[col * 8 + channel] as i32
                    + bottom[col * 8 + channel + 4] as i32
                    + 2)
                    / 4
            };
            [uv[col * 2], uv[col * 2 + 1]] = bt709_chroma(avg(0), avg(1), avg(2));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_unaligned_rows_preserve_output_guards() {
        let input = [
            99, 0, 0, 0, 255, 255, 255, 255, 0, 77, 255, 255, 255, 1, 0, 0, 0, 128,
        ];
        let mut output = [0xa5; 8];
        rgba_to_nv12_bt709(&input[1..], 2, 2, 9, &mut output[1..7]);
        assert_eq!(output, [0xa5, 16, 235, 235, 16, 128, 128, 0xa5]);
    }

    #[test]
    fn alpha_does_not_change_encoded_pixels() {
        let mut rgba = Vec::with_capacity(256 * 4);
        for alpha in 0..=255 {
            rgba.extend_from_slice(&[231, 59, 118, alpha]);
        }
        let mut output = vec![0; 128 * 3];
        rgba_to_nv12_bt709(&rgba, 128, 2, 128 * 4, &mut output);
        let luma = bt709_luma(231, 59, 118);
        assert!(output[..256].iter().all(|byte| *byte == luma));
        let chroma = bt709_chroma(231, 59, 118);
        assert!(output[256..].chunks_exact(2).all(|pair| pair == chroma));
    }

    #[test]
    fn empty_dimensions_do_not_access_buffers() {
        for (width, height) in [(0, 0), (0, 2), (2, 0)] {
            let mut output = [0xa5; 3];
            rgba_to_nv12_bt709(&[], width, height, 0, &mut output);
            assert_eq!(output, [0xa5; 3]);
        }
    }

    #[test]
    #[should_panic(expected = "RGBA frame is truncated")]
    fn truncated_rgba_is_rejected() {
        rgba_to_nv12_bt709(&[0; 63], 8, 2, 32, &mut [0; 24]);
    }

    #[test]
    #[should_panic(expected = "NV12 output is truncated")]
    fn truncated_nv12_is_rejected() {
        rgba_to_nv12_bt709(&[0; 64], 8, 2, 32, &mut [0; 23]);
    }

    #[test]
    #[should_panic(expected = "RGBA stride is shorter than a pixel row")]
    fn overlapping_rows_are_rejected() {
        rgba_to_nv12_bt709(&[0; 64], 8, 2, 31, &mut [0; 24]);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn overflowing_dimensions_are_rejected() {
        rgba_to_nv12_bt709(&[], u32::MAX, u32::MAX, u32::MAX, &mut []);
    }
}
