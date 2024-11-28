use std::{f32::consts::TAU, fs::File, io::Read, path::Path, sync::Arc, time::SystemTime};

use macroquad::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub dist: f32,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSettings {
    pub species_relations: Vec<Vec<Interaction>>,
    pub species: Vec<Vec4>,
    pub particle_size: f32,
    pub interaction_dist: f32,
    pub friction: f32,
}

impl SimulationSettings {
    pub fn from_file(path: impl AsRef<Path>) -> Result<SimulationSettings, anyhow::Error> {
        let mut str = String::new();

        let mut file = File::open(path)?;

        file.read_to_string(&mut str)?;

        let settings = serde_json::from_str::<SimulationSettings>(str.as_ref())?;

        Ok(settings)
    }

    pub fn random() -> Self {
        let mut settings = Self {
            species_relations: vec![],
            species: [
                Color::from_rgba(243, 139, 168, 255),
                Color::from_rgba(250, 179, 135, 255),
                Color::from_rgba(249, 226, 175, 255),
                Color::from_rgba(166, 227, 161, 255),
                Color::from_rgba(148, 226, 213, 255),
                Color::from_rgba(137, 180, 250, 255),
                Color::from_rgba(203, 166, 247, 255),
                // Color::from_rgba(24, 24, 37, 255),
            ]
            .into_iter()
            .map(|c| c.to_vec())
            .collect(),
            friction: 0.9,
            interaction_dist: 70.0,
            particle_size: 5.,
        };

        let count = settings.species.len();

        rand::srand(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        settings.species_relations = (0..count)
            .map(|_| {
                let mut v = vec![];

                for _ in 0..count {
                    v.push(Interaction {
                        dist: rand::gen_range(0., 100.),
                        strength: rand::gen_range(-2., 1f32),
                    })
                }
                v
            })
            .collect();

        settings
    }

    fn species_color(&self, species: Species) -> Option<Color> {
        self.species.get(species.0).copied().map(Color::from_vec)
    }

    pub fn interaction(&self, first: Species, second: Species) -> &Interaction {
        &self.species_relations[first.0][second.0]
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Species(usize);

#[derive(Debug, Clone)]
pub struct Particle {
    position: Vec2,
    velocity: Vec2,
    accel: Vec2,
    species: Species,
    settings: Arc<SimulationSettings>,
    bounds: (Vec2, Vec2),
}

impl Particle {
    pub fn random(settings: Arc<SimulationSettings>, min: Vec2, max: Vec2) -> Particle {
        Particle {
            species: Species(rand::gen_range(0, settings.species.len())),
            velocity: 100. * Vec2::from_angle(rand::gen_range(0., TAU)),
            position: vec2(rand::gen_range(min.x, max.x), rand::gen_range(min.y, max.y)),
            bounds: (min, max),
            accel: Vec2::ZERO,
            settings,
        }
    }

    pub fn interact(&mut self, dt: f32, other: &[Particle]) {
        let interact_dist = self.settings.interaction_dist;
        let interact_dist2 = interact_dist.powi(2);

        let (mut force, new_position, update_count) = other
            .par_iter()
            .map(|other| {
                let to_other = other.position - self.position;
                let dist = to_other.length_squared();

                if dist < 1E-3 {
                    return (Vec2::ZERO, None);
                }

                if dist > interact_dist2 {
                    return (Vec2::ZERO, None);
                }

                let dist = dist.sqrt();
                let dir = to_other / dist;

                let interaction = self.settings.interaction(self.species, other.species);

                let new_pos = if dist < self.settings.particle_size * 2. {
                    Some((self.position + to_other / 2.) - dir * self.settings.particle_size)
                } else {
                    None
                };

                let force = dir
                    * interaction.strength
                    * (interaction.dist - (dist - self.settings.particle_size * 2.))
                    / interact_dist;

                (force, new_pos)
            })
            .fold(
                || (Vec2::ZERO, Vec2::ZERO, 0usize),
                |(total_force, total_new_pos, update_count), (force, pos)| {
                    (
                        total_force + force,
                        total_new_pos + pos.clone().unwrap_or(Vec2::ZERO),
                        update_count + if pos.is_some() { 1 } else { 0 },
                    )
                },
            )
            .reduce(
                || (Vec2::ZERO, Vec2::ZERO, 0usize),
                |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
            );

        if update_count > 1 {
            let new_position = new_position / update_count as f32;
            force += (new_position - self.position) * 10.;
            self.position = new_position;
        }

        self.accel = 100. * force;
        self.velocity += self.accel * dt;
    }

    pub fn elliptic_space(mut position: Vec2, velocity: Vec2, min: Vec2, max: Vec2) -> Vec2 {
        fn modspace(x: &mut f32, v: f32, min: f32, max: f32) {
            if *x > max && v > 0. {
                *x -= max - min;
            } else if *x < min && v < 0. {
                *x += max - min;
            }
        }

        modspace(&mut position.x, velocity.x, min.x, max.x);
        modspace(&mut position.y, velocity.y, min.y, max.y);

        position
    }

    pub fn integrate(&mut self, dt: f32, mouse_pos: Option<Vec2>) {
        self.position += self.velocity * dt;

        self.position =
            Self::elliptic_space(self.position, self.velocity, self.bounds.0, self.bounds.1);
        // self.position = self.position.clamp(self.bounds.0, self.bounds.1);

        if let Some(mouse_pos) = mouse_pos {
            let diff = mouse_pos - self.position;
            if diff.length() < 200. {
                self.velocity += diff.normalize() * dt * diff.length_squared() * 1.0E-1;
            }
        }

        self.velocity += (-self.position).normalize() * self.position.length().sqrt() / 20.;
        self.velocity *= self.settings.friction;
    }

    pub fn draw_vel(&self) {
        let predicted = self.position + self.velocity * 0.1;

        draw_line(
            self.position.x,
            self.position.y,
            predicted.x,
            predicted.y,
            2.,
            Color::from_vec(BLACK.to_vec().lerp(
                WHITE.to_vec(),
                (self.velocity.length().sqrt() * 0.3).clamp(0., 1.),
            )),
        );
    }

    pub fn draw(&self) {
        draw_circle(
            self.position.x,
            self.position.y,
            self.settings.particle_size,
            self.settings.species_color(self.species).unwrap(),
        );
    }
}
