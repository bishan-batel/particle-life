mod particle;

use macroquad::prelude::*;
use rayon::prelude::*;

use particle::{Particle, SimulationSettings};
use std::{fs::File, io::Write, sync::Arc};

#[macroquad::main("BasicShapes")]
async fn main() -> Result<(), anyhow::Error> {
    const COUNT: usize = 1000;
    let settings = Arc::new(
        SimulationSettings::from_file("sim_settings.json")
            .unwrap_or_else(|_| SimulationSettings::random()),
    );

    let size = vec2(3200., 2000.) / 2.;
    request_new_screen_size(size.x, size.y);

    let mut particles: Vec<Particle> = (0..COUNT)
        .map(|_| Particle::random(settings.clone(), -size / 2., size / 2.))
        .collect();

    let mut show_vel_lines = true;

    let mut radius = 0.;

    loop {
        clear_background(color_u8!(24, 24, 37, 255));

        set_camera(&Camera2D {
            zoom: 2. / size,
            ..Default::default()
        });

        let dt = get_frame_time();

        // iterating through all particles,
        // cloning them
        // and giving each 'dt' and the list of all others
        //
        // and then collecting to a vec
        particles = particles
            // rayon will decide also how to divy up the work based off the system
            // there are still some differences tho
            .par_iter() // rayon's parallel iter
            .map(Clone::clone)
            .map(|mut p| {
                p.interact(dt, &particles);
                p
            })
            .collect();

        let mouse_pos = size * Vec2::from(mouse_position_local());
        let mouse_pos_opt = if is_mouse_button_down(MouseButton::Left) {
            Some(mouse_pos)
        } else {
            None
        };

        particles
            .par_iter_mut()
            .for_each(|p| p.integrate(dt, mouse_pos_opt));

        show_vel_lines ^= is_key_pressed(KeyCode::Key1);

        if show_vel_lines {
            particles.iter().for_each(Particle::draw_vel);
        }

        particles.iter().for_each(Particle::draw);

        radius = radius.lerp(if mouse_pos_opt.is_some() { 200. } else { 5. }, dt * 10.);
        draw_circle_lines(mouse_pos.x, mouse_pos.y, radius, 1., WHITE);

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
