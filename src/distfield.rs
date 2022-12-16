use ultraviolet::Vec3;

#[derive(Clone, Copy)]
pub struct Surface {
    pub color: Vec3,
    pub reflectivity: f32,
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
pub struct Sample {
    pub distance: f32,
    pub surface: Surface,
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

fn sphere(p: Vec3, center: Vec3, radius: f32, surface: Surface) -> Sample {
    // sphere at origin
    Sample {
        distance: (p - center).mag() - radius,
        surface,
    }
}

fn warp(p: Vec3) -> Vec3 {
    p + Vec3::new((0.4 * p.y).sin(), (0.6 * p.z).sin(), (0.8 * p.x).sin())
}

fn displace(p: Vec3, scale: f32, detail: f32, s: Sample) -> Sample {
    let p = p * detail;
    let displacement = scale * p.x.sin() * p.y.sin() * p.z.sin();
    Sample {
        distance: s.distance + displacement,
        ..s
    }
}

pub fn distfield(p: Vec3) -> Sample {
    let mat1 = Surface::new(Vec3::new(1.0, 0.8, 0.4), 0.4);
    let mat2 = Surface::new(Vec3::new(0.4, 0.8, 1.0), 0.2);
    let mat3 = Surface::new(Vec3::new(1.0, 0.4, 0.8), 0.0);
    intersect(
        union(
            sphere(warp(p), Vec3::new(-30., 0., 0.), 65., mat1),
            sphere(p, Vec3::new(30., 10., -10.), 50., mat2),
        ),
        invert(displace(
            p,
            10.,
            0.2,
            sphere(p, Vec3::new(10., -20., -60.), 30., mat3),
        )),
    )
}
