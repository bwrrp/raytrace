use nalgebra_glm::{clamp_scalar, distance2, dot, mix, normalize, reflect_vec, Vec3};

mod distfield;
use distfield::{distfield, Sample, Surface};

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pos: Vec3,
    color: Vec3,
}

impl Light {
    pub fn new(pos: Vec3, color: Vec3) -> Self {
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

fn raycast<F>(from: &Vec3, dir: &Vec3, condition: F) -> Option<(Sample, Vec3)>
where
    F: Fn(&Vec3) -> bool,
{
    let mut p = *from;
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
    let mut p = *from;
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

pub fn raytrace(from: &Vec3, dir: &Vec3, lights: &[Light], max_bounces: usize) -> Option<Vec3> {
    raycast(from, dir, |p| distance2(from, p) < 1000000.).map(|(s, p)| {
        let n = guess_normal(&p);
        let mut rgb = apply_lights(&p, &s.surface, &n, lights.iter());

        let reflectivity = s.surface.reflectivity;
        if reflectivity > 0.0 && max_bounces > 0 {
            let r = reflect_vec(dir, &n);
            let p = raycast_out(&p, &r);
            let reflected_color = raytrace(&p, &r, lights, max_bounces - 1)
                .unwrap_or_else(|| Vec3::new(0.3, 0.3, 0.3));
            rgb = mix(&rgb, &reflected_color, reflectivity);
        }
        rgb
    })
}
