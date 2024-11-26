mod particle;

use macroquad::{
    prelude::*,
    ui::{self, root_ui, widgets},
};

use miniquad::fs;
use particle::{Interaction, Particle, SimulationSettings};
use rayon::prelude::*;
use serde::Deserialize;
use std::{
    fs::File,
    io::{Read, Write},
    sync::Arc,
    time::SystemTime,
};

#[macroquad::main("BasicShapes")]
async fn main() -> Result<(), anyhow::Error> {
    const COUNT: usize = 1000;
    let settings = Arc::new({
        let mut str = String::new();

        File::open("sim_settings.json")
            .map(|mut f| f.read_to_string(&mut str))
            .map(move |_| str)
            .map_err(|x| anyhow::Error::from(x))
            .and_then(|x| {
                serde_json::from_str::<SimulationSettings>(x.as_ref()).map_err(anyhow::Error::from)
            })
            .unwrap_or_else(|_| {
                let mut settings = SimulationSettings {
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
                            // v.push(rand::gen_range(-1., 1.));
                            // v.push(1.);
                        }
                        v
                    })
                    .collect();

                settings
            })
    });

    let size = vec2(3200., 2000.) / 2.;
    request_new_screen_size(size.x, size.y);

    let mut particles: Vec<Particle> = (0..COUNT)
        .map(|_| Particle::random(settings.clone(), -size / 2., size / 2.))
        .collect();

    let mut show_vel_lines = true;

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

        let mouse_pos = if is_mouse_button_down(MouseButton::Left) {
            Some(size * Vec2::from(mouse_position_local()))
        } else {
            None
        };

        particles
            .par_iter_mut()
            .for_each(|p| p.integrate(dt, mouse_pos));

        show_vel_lines ^= is_key_pressed(KeyCode::Key1);

        if show_vel_lines {
            for particle in particles.iter() {
                particle.draw_vel()
            }
        }

        for particle in particles.iter() {
            particle.draw()
        }

        if let Some(pos) = mouse_pos {
            draw_circle_lines(pos.x, pos.y, 200., 1., WHITE);
        }

        if is_key_pressed(KeyCode::S) && is_key_down(KeyCode::LeftControl) {
            if let Err(err) = save_settings(&settings) {
                error!("{}", err);
            }
        }

        next_frame().await
    }
}

fn save_settings(settings: &SimulationSettings) -> Result<(), anyhow::Error> {
    let string = serde_json::to_string_pretty(settings)?;
    let mut file = File::create("sim_settings.json")?;
    file.write_fmt(format_args!("{}", string))?;
    Ok(())
}
