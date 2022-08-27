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

const STAGE_SIZE: IVec2 = IVec2::from_array([40,40]);
const ACCELERATION: f32 = 0.35;
const MAX_SPEED: f32 = 35.;
const BODY_GLYPH: char = '█';
const FOOD_GLYPH: char = '☼';

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TerminalPlugin)
        .add_plugin(WindowPlugin)
        .add_startup_system(setup)
        .add_system(make_food)
        .add_system(drive.after(make_food))
        .add_system(eat.after(drive))
        .add_system(grow.after(eat))
        .add_system(render.after(grow))
        .add_system(die.after(render))
        .run();
}

#[derive(Component)]
pub struct Food {
    pos: IVec2,
}

#[derive(Component)]
struct GridPos(IVec2);

#[derive(Component)]
struct Steering {
    cell_pos: f32,
    dir: IVec2,
    prev: IVec2,
    speed: f32,
}

#[derive(Component)]
struct Body(VecDeque<IVec2>);

#[derive(Component)]
struct Grow {
    turns: usize,
    pos: IVec2,
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
        prev: IVec2::ZERO,
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

        if dir != IVec2::ZERO && pos.0 + dir != steering.prev {
            steering.dir = dir;
        }

        steering.cell_pos += steering.speed * dt;

        if steering.cell_pos < 1.0 {
            continue;
        }

        steering.cell_pos -= 1.0;
        let body = &mut body.0;
        let next = *body.front().unwrap() + steering.dir;
        steering.prev = pos.0;
        body.push_front(next);
        body.pop_back();

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
            let body = &body.0;
            loop {
                let pos = rand_pos(&mut rng, STAGE_SIZE);
                let pos = pos - STAGE_SIZE / 2;

                if body.contains(&pos) || !in_bounds(pos) {
                    continue;
                }

                commands.spawn().insert(Food {
                    pos,
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
    mut q_term: Query<(&mut Terminal, &ToWorld)>,
    q_snake: Query<&Body, Changed<Body>>,
    q_food: Query<&Food>,
) {
    if let Ok(body) = q_snake.get_single() {
        let body = &body.0;
        let (mut term, tw) = q_term.single_mut();

        term.clear();
        term.draw_border(BorderGlyphs::single_line());
        for food in &q_food {
            // Add one to account for borders
            let pos = food.pos + STAGE_SIZE / 2;
            term.put_char(pos, FOOD_GLYPH);
        }
        for pos in body.iter() {
            let pos = *pos + STAGE_SIZE / 2;
            term.put_char(pos, BODY_GLYPH);
        }
    }
}

fn eat(
    q_food: Query<(Entity, &Food)>,
    mut q_snake: Query<(&Body, &mut Steering, &GridPos), Changed<GridPos>>,
    mut commands: Commands,
) {
    for (body, mut steering, pos) in &mut q_snake {
        for (e_food, food) in &q_food {
            //println!("Snake pos {}, food pos {}", pos.0, food.pos);
            if pos.0 == food.pos {
                commands.entity(e_food).despawn();
                steering.speed = (steering.speed + ACCELERATION).min(MAX_SPEED);
                commands.spawn().insert(Grow {
                    turns: body.0.len(),
                    pos: pos.0,
                });
            }
        } 
    }
}

fn grow(
    mut q_grow: Query<(Entity, &mut Grow)>,
    mut q_snake: Query<(&mut Body, &GridPos), Changed<GridPos>>, 
    mut commands: Commands,
) {
    if q_snake.is_empty() {
        return;
    }

    for _ in &q_snake {
        for (_, mut grow) in &mut q_grow {
            grow.turns -= 1;
        }
    }

    let mut body = q_snake.single_mut().0;
    for (entity, grow) in &q_grow {
        if grow.turns != 0 {
            continue;
        }
        body.0.push_back(grow.pos);
        commands.entity(entity).despawn();
    }
}

fn die(
    q_snake: Query<(Entity, &GridPos, &Body), Changed<GridPos>>,
    q_food: Query<Entity, With<Food>>,
    mut q_term: Query<&mut Terminal>,
    mut commands: Commands,
) {
    let mut game_over = |entity| {
        commands.entity(entity).despawn();
        q_food.for_each(|e|commands.entity(e).despawn());
        let mut term = q_term.single_mut();
        term.clear();
        term.put_string([-5,1].pivot(Pivot::Center), "Game Over!");
    };

    if let Ok((snake_entity, pos, body)) = q_snake.get_single() {
        if !in_bounds(pos.0) {
            game_over(snake_entity);
        }

        for p in body.0.iter().skip(1) {
            if *p == pos.0 {
                game_over(snake_entity);
            }
        }
    }
}

fn in_bounds(p: IVec2) -> bool {
    let half_stage = STAGE_SIZE / 2;

    !(p.cmple(-half_stage).any() || p.cmpge(half_stage).any())
}