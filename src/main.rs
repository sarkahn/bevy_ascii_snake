// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod window;

use std::collections::VecDeque;

use bevy::prelude::{*};
use bevy::DefaultPlugins;
use bevy_ascii_terminal::{prelude::*, code_page_437, ToWorld};
use rand::Rng;
use rand::rngs::ThreadRng;
use window::WindowPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TerminalPlugin)
        .add_plugin(WindowPlugin)
        .add_startup_system(setup)
        .add_system(drive)
        .add_system(render)
        .add_system(make_food)
        .add_system(eat)
        .run();
}

#[derive(Component)]
pub struct Food {
    pos: IVec2,
    glyph: char,
}

#[derive(Component)]
struct GridPos(IVec2);

impl std::ops::DerefMut for GridPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for GridPos {
    type Target = IVec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component)]
struct Steering {
    cell_pos: f32,
    dir: IVec2,
    speed: f32,
}

#[derive(Component)]
struct Body(VecDeque<IVec2>);

impl std::ops::DerefMut for Body {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for Body {
    type Target = VecDeque<IVec2>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn setup(
    mut commands: Commands,
) {
    let mut term = Terminal::with_size([42,42]);
    term.draw_border(BorderGlyphs::single_line());

    commands.spawn_bundle(TerminalBundle::from(term))
    .insert(ToWorld::default())
    .insert(AutoCamera);

    let body = Body(VecDeque::from(vec![IVec2::ZERO]));
    let steering = Steering {
        cell_pos: 0.5,
        dir: [0,1].into(),
        speed: 5.0,
    };
    let grid_pos = GridPos([0,0].into());
    commands.spawn().insert(body).insert(steering).insert(grid_pos);
}

fn drive(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut q_snake: Query<(&mut Body, &mut Steering, &mut GridPos)>
) {
    let dt = time.delta_seconds();

    for (mut body, mut steering, mut pos) in &mut q_snake {

        let mut dir = IVec2::ZERO;
        if input.just_pressed(KeyCode::W) {
            dir.y = 1;
        }

        if input.just_pressed(KeyCode::S) {
            dir.y = -1;
        }

        if input.just_pressed(KeyCode::A) {
            dir.x = -1;
        }

        if input.just_pressed(KeyCode::D) {
            dir.x = 1;
        }

        if dir != IVec2::ZERO && dir != -steering.dir {
            steering.dir = dir;
        }

        steering.cell_pos += steering.speed * dt;

        if steering.cell_pos < 1.0 {
            continue;
        }

        steering.cell_pos -= 1.0;
        let next = *body.back().unwrap() + steering.dir;
        body.push_back(next);
        body.pop_front();

        *pos = GridPos(next);
    }
}

fn make_food(
    mut commands: Commands,
    q_food: Query<&Food>,
    q_body: Query<&Body>,
) {
    let mut rng = ThreadRng::default();
    if q_food.is_empty() {
        if let Ok(body) = q_body.get_single() {
            loop {
                let pos = rand_pos(&mut rng, IVec2::splat(40));
                
                if body.contains(&pos) {
                    continue;
                }

                commands.spawn().insert(Food {
                    pos,
                    glyph: '☼',
                });
                break;
            }
        }
    }
}

fn rand_pos(rng: &mut ThreadRng, dimensions: IVec2) -> IVec2 {
    let x = rng.gen_range(0..dimensions.x);
    let y = rng.gen_range(0..dimensions.y);
    IVec2::new(x,y)
}

fn render(
    mut q_term: Query<&mut Terminal>,
    q_snake: Query<&Body, Changed<Body>>,
    q_food: Query<&Food>,
) {
    if let Ok(body) = q_snake.get_single() {
        let mut term = q_term.single_mut();
        term.clear();
        term.draw_border(BorderGlyphs::single_line());
        for food in &q_food {
            // Add one to account for borders
            term.put_char(food.pos + IVec2::ONE, food.glyph);
        }
        for point in body.iter() {
            term.put_char((*point).pivot(Pivot::Center), '█');
        }
    }
}

fn eat(
    q_food: Query<(Entity,&Food)>,
    q_snake: Query<(&Body, &Steering, &GridPos), Changed<GridPos>>,
    mut commands: Commands,
) {
    for (body, steering, pos) in &q_snake {
        for (e_food, food) in &q_food {
            if pos.0 == food.pos {
                commands.entity(e_food).despawn();
            }
        } 
    }
}