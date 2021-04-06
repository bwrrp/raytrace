use std::sync::{Arc, Mutex};

use anyhow::Result;
use image::{ImageBuffer, Rgba, RgbaImage};
use nalgebra_glm::{normalize, Vec3};
use rayon::prelude::*;

use raycast::{raytrace, Light};

fn main() -> Result<()> {
    let width = 640u32;
    let height = 480u32;

    let eye = Vec3::new(0., 0., -100.);
    let center = Vec3::new(width as _, height as _, 0.0) * 0.5;

    let mut img: RgbaImage = ImageBuffer::new(width, height);
    let coords: Vec<_> = img.enumerate_pixels().map(|(x, y, _)| (x, y)).collect();

    let lights = [
        Light::new(Vec3::new(500., 1000., -300.), Vec3::new(1.0, 0.5, 0.)),
        Light::new(Vec3::new(-700., -500., -10.), Vec3::new(0., 0.5, 1.0)),
        Light::new(Vec3::new(-700., 1500., 10.), Vec3::new(0.5, 0., 1.0)),
        Light::new(Vec3::new(10., -20., -50.), Vec3::new(0.3, 0.2, 0.2)),
    ];

    let progress = Arc::new(Mutex::new((0i32, progress::Bar::new())));
    let pixels: Vec<_> = coords
        .par_iter()
        .map_with(progress, |progress, (x, y)| {
            {
                let mut progress = progress.lock().unwrap();
                let (ref mut num, ref mut bar) = *progress;
                *num += 1;
                if *num % 16 == 0 {
                    bar.reach_percent(*num * 100 / (width * height) as i32);
                }
            }

            let p_img = Vec3::new(*x as _, (height - *y) as _, 0.0);
            let p_scaled = (p_img - center) / width.min(height) as f32 * 250.;
            let ray_dir = normalize(&(p_scaled - eye));

            let color = raytrace(&eye, &ray_dir, &lights, 5).map(|rgb| {
                let rgb_scaled = rgb * 255.;
                Rgba([rgb_scaled.x as _, rgb_scaled.y as _, rgb_scaled.z as _, 255])
            });

            color.unwrap_or(Rgba([0, 0, 0, 0]))
        })
        .collect();

    img.enumerate_pixels_mut()
        .map(|(_, _, pixel)| pixel)
        .zip(pixels)
        .for_each(|(pixel, color)| {
            *pixel = color;
        });

    Ok(img.save("test.png")?)
}
