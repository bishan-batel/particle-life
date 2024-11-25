use std::{f32::consts::TAU, sync::Arc};

use macroquad::{input, prelude::*};

#[derive(Debug, Clone)]
pub struct Interaction {
    pub dist: f32,
    pub strength: f32,
}

#[derive(Debug, Clone)]
pub struct SimulationSettings {
    pub species_relations: Vec<Vec<Interaction>>,
    pub species: Vec<Color>,
    pub particle_size: f32,
    pub interaction_dist: f32,
    pub friction: f32,
}

impl SimulationSettings {
    fn species_color(&self, species: Species) -> Option<Color> {
        self.species.get(species.0).copied()
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

        let mut force = vec2(0., 0.);

        let mut new_position = Vec2::ZERO;
        let mut update_count: usize = 0;

        for other in other.iter() {
            let to_other = other.position - self.position;
            let dist = to_other.length_squared();

            if dist < 1E-3 {
                continue;
            }

            if dist > interact_dist2 {
                continue;
            }

            let dist = dist.sqrt();
            let dir = to_other / dist;

            let interaction = self.settings.interaction(self.species, other.species);

            force += dir
                * interaction.strength
                * (interaction.dist - (dist - self.settings.particle_size * 2.))
                / interact_dist;

            if dist < self.settings.particle_size * 2. {
                new_position +=
                    (self.position + to_other / 2.) - dir * self.settings.particle_size * 1.1;
                update_count += 1;
            }
        }

        if update_count > 1 {
            new_position /= update_count as f32;
            force += (new_position - self.position) * 10.;
            self.position = new_position;
        }

        self.accel = 100. * force;
        self.velocity += self.accel * dt;
    }

    pub fn integrate(&mut self, dt: f32, mouse_pos: Option<Vec2>) {
        self.position += self.velocity * dt;

        let modspace = |x: &mut f32, v: f32, min: f32, max: f32| {
            if *x > max && v > 0. {
                *x -= max - min;
            } else if *x < min && v < 0. {
                *x += max - min;
            }
        };

        modspace(
            &mut self.position.x,
            self.velocity.x,
            self.bounds.0.x,
            self.bounds.1.x,
        );

        modspace(
            &mut self.position.y,
            self.velocity.y,
            self.bounds.0.y,
            self.bounds.1.y,
        );

        // self.position = self.position.clamp(self.bounds.0, self.bounds.1);

        if let Some(mouse_pos) = mouse_pos {
            let diff = mouse_pos - self.position;
            if diff.length() < 200. {
                self.velocity += diff.normalize() * dt * 500.;
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

    pub fn species(&self) -> Species {
        self.species
    }
}
