mod particle;

use core::time;
use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use humantime::Duration;
use macroquad::{color::colors, input, prelude::*};
use particle::{Interaction, Particle, SimulationSettings};
use rayon::prelude::*;

#[macroquad::main("BasicShapes")]
async fn main() -> Result<(), anyhow::Error> {
    fern::Dispatch::new().chain(std::io::stdout()).apply()?;

    const COUNT: usize = 2000;
    let settings = Arc::new({
        let mut settings = SimulationSettings {
            species_relations: vec![],
            species: vec![
                color_u8!(243, 139, 168, 255),
                color_u8!(235, 160, 172, 255),
                color_u8!(249, 226, 175, 255),
                color_u8!(166, 227, 161, 255),
                color_u8!(137, 180, 250, 255),
                color_u8!(203, 166, 247, 255),
            ],
            friction: 0.94,
            interaction_dist: 100.0,
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
                    // v.push(rand::gen_range(-1., 1.));
                    // v.push(1.);
                }
                v
            })
            .collect();

        settings
    });

    let size = vec2(3200., 2000.) / 2.;
    request_new_screen_size(size.x, size.y);

    let mut particles: Vec<Particle> = (0..COUNT)
        .map(|_| Particle::random(settings.clone(), -size / 2., size / 2.))
        .collect();

    loop {
        clear_background(color_u8!(24, 24, 37, 255));

        set_camera(&Camera2D {
            zoom: 2. / size,
            ..Default::default()
        });

        let dt = get_frame_time();

        particles = particles
            .par_iter()
            .map(Clone::clone)
            .map(|mut p| {
                p.interact(dt, &particles);
                p
            })
            .collect();

        let mouse_pos = if input::is_mouse_button_down(MouseButton::Left) {
            Some(size * Vec2::from(mouse_position_local()))
        } else {
            None
        };

        particles
            .par_iter_mut()
            .for_each(|p| p.integrate(dt, mouse_pos));

        for particle in particles.iter() {
            particle.draw_vel()
        }

        for particle in particles.iter() {
            particle.draw()
        }

        if let Some(pos) = mouse_pos {
            draw_circle_lines(pos.x, pos.y, 200., 1., WHITE);
        }

        next_frame().await
    }
}
