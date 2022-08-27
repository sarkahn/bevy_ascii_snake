// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod window;

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::DefaultPlugins;
use bevy_ascii_terminal::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioPlugin, AudioSource};
use rand::rngs::ThreadRng;
use rand::Rng;
use window::WindowPlugin;

const STAGE_SIZE: IVec2 = IVec2::from_array([40, 36]);
const START_SPEED: f32 = 8.0;
const ACCELERATION: f32 = 0.35;
const MAX_SPEED: f32 = 35.;
const BODY_GLYPH: char = '█';
const FOOD_GLYPH: char = '☼';

#[derive(Debug, StageLabel, Clone, Eq, PartialEq, Hash)]
enum GameState {
    Begin,
    Playing,
}

fn main() {
    App::new()
        .add_plugin(WindowPlugin)
        .add_plugins(DefaultPlugins)
        .add_plugin(TerminalPlugin)
        .add_plugin(AudioPlugin)
        .init_resource::<FoodCount>()
        .init_resource::<Sounds>()
        .add_state(GameState::Begin)
        .add_startup_system(setup)
        .add_system_set(SystemSet::on_update(GameState::Begin).with_system(start))
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(spawn))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(make_food)
                .with_system(drive.after(make_food))
                .with_system(eat.after(drive))
                .with_system(grow.after(eat))
                .with_system(render.after(grow))
                .with_system(die.after(render)),
        )
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

#[derive(Default)]
struct FoodCount(usize);

#[derive(Default)]
struct Sounds {
    nom: Handle<AudioSource>,
    ouch: Handle<AudioSource>,
    ding: Handle<AudioSource>,
}

fn setup(mut commands: Commands, server: Res<AssetServer>, mut sfx: ResMut<Sounds>) {
    let mut term = Terminal::with_size(STAGE_SIZE + 2);
    term.draw_border(BorderGlyphs::single_line());
    term.draw_box(
        [0, 5].pivot(Pivot::Center),
        [13, 3],
        UiBox::double_line().color_fill(Color::GRAY, Color::BLACK),
    );
    term.put_string([-5, 5].pivot(Pivot::Center), "ASCII SNAKE".fg(Color::BLUE));
    term.put_string([-6, 2].pivot(Pivot::Center), "Use WASD to move");
    term.put_string([-9, 1].pivot(Pivot::Center), "Press Space to Begin");

    commands
        .spawn_bundle(TerminalBundle::from(term))
        .insert(AutoCamera);

    sfx.nom = server.load("nom.wav");
    sfx.ouch = server.load("ouch.wav");
    sfx.ding = server.load("ding.wav");
}

fn start(
    input: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
    audio: Res<Audio>,
    sfx: Res<Sounds>,
) {
    if input.just_pressed(KeyCode::Space) {
        state.set(GameState::Playing).unwrap();
        audio.play(sfx.ding.clone());
    }
}

fn spawn(mut commands: Commands, mut count: ResMut<FoodCount>) {
    let body = Body(VecDeque::from(vec![IVec2::ZERO]));
    let steering = Steering {
        cell_pos: 0.5,
        dir: [0, 1].into(),
        speed: START_SPEED,
        prev: IVec2::ZERO,
    };
    let grid_pos = GridPos([0, 0].into());
    commands
        .spawn()
        .insert(body)
        .insert(steering)
        .insert(grid_pos);
    count.0 = 0;
}

fn drive(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut q_snake: Query<(&mut Body, &mut Steering, &mut GridPos)>,
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

fn make_food(mut commands: Commands, q_food: Query<&Food>, q_body: Query<&Body>) {
    let mut rng = ThreadRng::default();
    if q_food.is_empty() {
        if let Ok(body) = q_body.get_single() {
            let body = &body.0;
            loop {
                let x = rng.gen_range(0..STAGE_SIZE.x);
                let y = rng.gen_range(0..STAGE_SIZE.y);
                let pos = IVec2::new(x, y) - STAGE_SIZE / 2;

                if body.contains(&pos) || !in_bounds(pos) {
                    continue;
                }

                commands.spawn().insert(Food { pos });
                break;
            }
        }
    }
}

fn render(
    mut q_term: Query<&mut Terminal>,
    q_snake: Query<&Body, Changed<Body>>,
    q_food: Query<&Food>,
) {
    if let Ok(body) = q_snake.get_single() {
        let body = &body.0;
        let mut term = q_term.single_mut();

        term.clear();
        term.draw_border(BorderGlyphs::single_line());
        for food in &q_food {
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
    mut count: ResMut<FoodCount>,
    audio: Res<Audio>,
    sfx: Res<Sounds>,
) {
    for (body, mut steering, pos) in &mut q_snake {
        for (e_food, food) in &q_food {
            if pos.0 == food.pos {
                count.0 += 1;
                commands.entity(e_food).despawn();
                steering.speed = (steering.speed + ACCELERATION).min(MAX_SPEED);
                commands.spawn().insert(Grow {
                    turns: count.0,
                    pos: *body.0.back().unwrap(),
                });
                audio.play(sfx.nom.clone());
            }
        }
    }
}

fn grow(
    mut q_grow: Query<(Entity, &mut Grow)>,
    mut q_snake: Query<(&mut Body, &GridPos), Changed<GridPos>>,
    mut commands: Commands,
    count: Res<FoodCount>,
) {
    if q_snake.is_empty() {
        return;
    }

    let mut body = q_snake.single_mut().0;
    for (entity, mut grow) in &mut q_grow {
        if grow.turns <= count.0 {
            body.0.push_back(grow.pos);
        }

        grow.turns -= 1;

        if grow.turns == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn die(
    q_snake: Query<(Entity, &GridPos, &Body), Changed<GridPos>>,
    q_food: Query<Entity, With<Food>>,
    mut q_term: Query<&mut Terminal>,
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    audio: Res<Audio>,
    sfx: Res<Sounds>,
) {
    let mut game_over = |entity| {
        commands.entity(entity).despawn();
        q_food.for_each(|e| commands.entity(e).despawn());
        let mut term = q_term.single_mut();
        term.clear();
        term.put_string([-4, 1].pivot(Pivot::Center), "Game Over!");
        term.put_string([-12, 0].pivot(Pivot::Center), "Press Spacebar to restart");
        state.set(GameState::Begin).unwrap();
        audio.play(sfx.ouch.clone());
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

    !(p.cmple(-half_stage).any() || p.cmpge(half_stage + 1).any())
}
