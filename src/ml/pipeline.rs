use ndarray::{Array, Array4};
use image::{DynamicImage, GenericImageView};
use anyhow::Result;

pub fn normalize_for_nsfw(image: &DynamicImage) -> Result<Array4<f32>> {
    let resized = image.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
    let mut array = Array::zeros((1, 3, 224, 224));

    for (x, y, pixel) in resized.pixels() {
        let r = (pixel[0] as f32 / 255.0 - 0.5) / 0.5;
        let g = (pixel[1] as f32 / 255.0 - 0.5) / 0.5;
        let b = (pixel[2] as f32 / 255.0 - 0.5) / 0.5;

        array[[0, 0, y as usize, x as usize]] = r;
        array[[0, 1, y as usize, x as usize]] = g;
        array[[0, 2, y as usize, x as usize]] = b;
    }

    Ok(array)
}

pub fn normalize_for_tagger(image: &DynamicImage) -> Result<Array4<f32>> {
    // Tagger: Resize to 448x448. Normalize by dividing pixel values by 255.0 (0.0-1.0 range).
    let resized = image.resize_exact(448, 448, image::imageops::FilterType::Lanczos3);
    let mut array = Array::zeros((1, 3, 448, 448));

    // Note: Some tagger models expect BGR or different normalization,
    // but the prompt specifies "Normalize by dividing pixel values by 255.0".
    // It doesn't specify mean/std subtraction for tagger.

    for (x, y, pixel) in resized.pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;

        array[[0, 0, y as usize, x as usize]] = r;
        array[[0, 1, y as usize, x as usize]] = g;
        array[[0, 2, y as usize, x as usize]] = b;
    }

    Ok(array)
}
