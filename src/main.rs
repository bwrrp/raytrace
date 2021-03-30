use std::sync::{Arc, Mutex};

use anyhow::Result;
use image::{ImageBuffer, Rgba, RgbaImage};
use nalgebra_glm::{clamp_scalar, distance, distance2, dot, mix, normalize, reflect_vec, Vec3};
use progress;
use rayon::prelude::*;

#[derive(Clone, Copy)]
struct Surface {
    color: Vec3,
    reflectivity: f32,
}

impl Surface {
    fn new(color: Vec3, reflectivity: f32) -> Self {
        Self {
            color,
            reflectivity,
        }
    }
}

#[derive(Clone, Copy)]
struct Sample {
    distance: f32,
    surface: Surface,
}

fn union(s1: Sample, s2: Sample) -> Sample {
    if s1.distance < s2.distance {
        s1
    } else {
        s2
    }
}

fn intersect(s1: Sample, s2: Sample) -> Sample {
    if s1.distance < s2.distance {
        s2
    } else {
        s1
    }
}

fn invert(s: Sample) -> Sample {
    Sample {
        distance: -1.0 * s.distance,
        surface: s.surface,
    }
}

fn sphere(p: &Vec3, center: &Vec3, radius: f32, surface: Surface) -> Sample {
    // sphere at origin
    Sample {
        distance: distance(p, center) - radius,
        surface,
    }
}

fn warp(p: &Vec3) -> Vec3 {
    p + Vec3::new((0.4 * p.y).sin(), (0.6 * p.z).sin(), (0.8 * p.x).sin())
}

fn displace(p: &Vec3, scale: f32, detail: f32, s: Sample) -> Sample {
    let p = p * detail;
    let displacement = scale * p.x.sin() * p.y.sin() * p.z.sin();
    Sample {
        distance: s.distance + displacement,
        ..s
    }
}

fn distfield(p: &Vec3) -> Sample {
    let mat1 = Surface::new(Vec3::new(1.0, 0.8, 0.4), 0.4);
    let mat2 = Surface::new(Vec3::new(0.4, 0.8, 1.0), 0.2);
    let mat3 = Surface::new(Vec3::new(1.0, 0.4, 0.8), 0.0);
    intersect(
        union(
            sphere(&warp(p), &Vec3::new(-30., 0., 0.), 65., mat1),
            sphere(p, &Vec3::new(30., 10., -10.), 50., mat2),
        ),
        invert(displace(
            p,
            10.,
            0.2,
            sphere(p, &Vec3::new(10., -20., -60.), 30., mat3),
        )),
    )
}

fn guess_normal(p: &Vec3) -> Vec3 {
    let delta = 0.01;
    let dx = Vec3::new(delta, 0., 0.);
    let dy = Vec3::new(0., delta, 0.);
    let dz = Vec3::new(0., 0., delta);
    normalize(&Vec3::new(
        (distfield(&(p + dx)).distance - distfield(&(p - dx)).distance) / (delta * 2.0),
        (distfield(&(p + dy)).distance - distfield(&(p - dy)).distance) / (delta * 2.0),
        (distfield(&(p + dz)).distance - distfield(&(p - dz)).distance) / (delta * 2.0),
    ))
}

fn raycast<F>(from: &Vec3, dir: &Vec3, condition: F) -> Option<(Sample, Vec3)>
where
    F: Fn(&Vec3) -> bool,
{
    let mut p = from.clone();
    while condition(&p) {
        let s = distfield(&p);
        if s.distance <= 0. {
            return Some((s, p));
        }
        let step = if s.distance > 0.01 { s.distance } else { 0.01 };
        p += dir * step;
    }
    None
}

fn raycast_out(from: &Vec3, dir: &Vec3) -> Vec3 {
    let mut p = from.clone();
    loop {
        let f = -1.0 * distfield(&p).distance;
        if f < 0. {
            break;
        }
        let step = if f > 0.01 { f } else { 0.01 };
        p += dir * step;
    }
    p
}

#[derive(Clone, Copy, Debug)]
struct Light {
    pos: Vec3,
    color: Vec3,
}

impl Light {
    fn new(pos: Vec3, color: Vec3) -> Self {
        Self { pos, color }
    }

    fn in_shadow(&self, point: &Vec3) -> bool {
        let l = normalize(&(self.pos - point));
        // Step out of object
        let p = raycast_out(point, &l);
        // Check for any objects while tracing towards the light source
        raycast(&p, &l, |p| dot(&(self.pos - p), &l) > 0.).is_some()
    }

    fn diffuse(&self, p: &Vec3, n: &Vec3) -> f32 {
        let l = normalize(&(self.pos - p));
        clamp_scalar(dot(&n, &l), 0., 1.)
    }
}

fn apply_lights<'a>(
    p: &Vec3,
    s: &Surface,
    n: &Vec3,
    lights: impl Iterator<Item = &'a Light>,
) -> Vec3 {
    let mut rgb = Vec3::new(0., 0., 0.);
    for light in lights {
        if !light.in_shadow(&p) {
            rgb += light.color.component_mul(&s.color) * light.diffuse(&p, &n);
        }
    }
    rgb
}

fn raytrace(from: &Vec3, dir: &Vec3, lights: &[Light], max_bounces: usize) -> Option<Vec3> {
    raycast(from, dir, |p| distance2(from, p) < 1000000.).map(|(s, p)| {
        let n = guess_normal(&p);
        let mut rgb = apply_lights(&p, &s.surface, &n, lights.iter());

        let reflectivity = s.surface.reflectivity;
        if reflectivity > 0.0 && max_bounces > 0 {
            let r = reflect_vec(dir, &n);
            let p = raycast_out(&p, &r);
            let reflected_color =
                raytrace(&p, &r, lights, max_bounces - 1).unwrap_or(Vec3::new(0.3, 0.3, 0.3));
            rgb = mix(&rgb, &reflected_color, reflectivity);
        }
        rgb
    })
}

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
