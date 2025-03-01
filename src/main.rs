use std::collections::VecDeque;
use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;
use bevy_ascii_terminal::*;
use rand::Rng;
use rand::rngs::ThreadRng;

const STAGE_SIZE: UVec2 = UVec2::from_array([20, 20]);
const START_DIR: IVec2 = IVec2::Y;
const BODY_GLYPH: char = '█';
const FOOD_GLYPH: char = '☼';

const INITIAL_TICK_DELAY: f32 = 0.15;
const ACCELERATION: f32 = 0.01;
const MIN_TICK_DELAY: f32 = 0.05;

#[derive(Event)]
struct Restart;

#[derive(Resource)]
struct DingSound(Handle<AudioSource>);

#[derive(Resource)]
struct NomSound(Handle<AudioSource>);

#[derive(Resource)]
struct OuchSound(Handle<AudioSource>);

#[derive(Resource, Deref, DerefMut)]
struct TickRate(Timer);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            TerminalPlugins,
        ))
        .init_resource::<FoodCount>()
        .insert_resource(TickRate(Timer::new(
            Duration::from_secs_f32(INITIAL_TICK_DELAY),
            TimerMode::Repeating,
        )))
        .add_event::<Restart>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                spawn.run_if(on_event::<Restart>),
                make_food,
                input,
                vroom,
                grow,
                eat,
                die,
            )
                .chain(),
        )
        .add_systems(PostUpdate, render)
        .run();
}

#[derive(Component)]
pub struct Food {
    pos: IVec2,
}

#[derive(Component, Deref, DerefMut)]
struct GridPos(IVec2);

#[derive(Component)]
struct GameState {
    curr_dir: IVec2,
    next_dir: IVec2,
}

#[derive(Component, Deref, DerefMut)]
struct Body(VecDeque<IVec2>);

#[derive(Component)]
struct Grow {
    turns: usize,
    pos: IVec2,
}

#[derive(Default, Resource, Deref, DerefMut)]
struct FoodCount(usize);

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let mut term = Terminal::new(STAGE_SIZE + 2);
    term.put_string([0, 2].pivot(Pivot::Center), "ASCII SNAKE".fg(color::BLUE));
    term.put_string([0, 1].pivot(Pivot::Center), "Use WASD to move");
    term.put_string([0, 0].pivot(Pivot::Center), "Press Space to Begin");

    commands.insert_resource(DingSound(server.load("ding.wav")));
    commands.insert_resource(NomSound(server.load("nom.wav")));
    commands.insert_resource(OuchSound(server.load("ouch.wav")));

    commands.spawn((term, TerminalBorder::single_line()));
    commands.spawn(TerminalCamera::new());
}

fn spawn(
    mut commands: Commands,
    mut count: ResMut<FoodCount>,
    ding: Res<DingSound>,
    mut tick: ResMut<TickRate>,
) {
    let body = Body(VecDeque::from(vec![IVec2::ZERO]));
    let state = GameState {
        curr_dir: START_DIR,
        next_dir: START_DIR,
    };
    let grid_pos = GridPos([0, 0].into());
    commands.spawn((body, state, grid_pos));
    count.0 = 0;
    commands.spawn((AudioPlayer::new(ding.0.clone()), PlaybackSettings::DESPAWN));
    tick.0
        .set_duration(Duration::from_secs_f32(INITIAL_TICK_DELAY));
}

fn input(
    input: Res<ButtonInput<KeyCode>>,
    mut q_snake: Query<&mut GameState>,
    mut restart: EventWriter<Restart>,
) {
    let Ok(mut state) = q_snake.get_single_mut() else {
        if input.just_pressed(KeyCode::Space) {
            restart.send(Restart);
        }
        return;
    };
    let left = [KeyCode::KeyA, KeyCode::ArrowLeft];
    let right = [KeyCode::KeyD, KeyCode::ArrowRight];
    let up = [KeyCode::KeyW, KeyCode::ArrowUp];
    let down = [KeyCode::KeyS, KeyCode::ArrowDown];

    let hor = input.any_pressed(right) as i32 - input.any_pressed(left) as i32;
    let ver = input.any_pressed(up) as i32 - input.any_pressed(down) as i32;

    if hor == 0 && ver == 0 {
        return;
    }
    state.next_dir = [hor, if hor == 0 { ver } else { 0 }].into();
}

fn vroom(
    mut q_snake: Query<(&mut Body, &mut GameState, &mut GridPos)>,
    time: Res<Time>,
    mut tick: ResMut<TickRate>,
) {
    tick.tick(time.delta());

    if tick.finished() {
        tick.reset();
        for (mut body, mut state, mut pos) in &mut q_snake {
            if state.next_dir != -state.curr_dir {
                state.curr_dir = state.next_dir;
            }

            let next = body.front().unwrap() + state.curr_dir;
            body.push_front(next);
            body.pop_back();
            *pos = GridPos(next);
        }
    }
}

fn make_food(mut commands: Commands, q_food: Query<&Food>, q_body: Query<&Body>) {
    let mut rng = ThreadRng::default();
    if q_food.is_empty() {
        if let Ok(body) = q_body.get_single() {
            let body = &body.0;
            loop {
                let size = STAGE_SIZE.as_ivec2();
                let x = rng.gen_range(0..size.x);
                let y = rng.gen_range(0..size.y);
                let pos = IVec2::new(x, y) - size / 2;

                if body.contains(&pos) {
                    continue;
                }

                commands.spawn(Food { pos });
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
    let mut term = q_term.single_mut();
    if let Ok(body) = q_snake.get_single() {
        let body = &body.0;

        term.clear();
        for food in &q_food {
            let pos = food.pos + STAGE_SIZE.as_ivec2() / 2;
            term.put_char(pos, FOOD_GLYPH);
        }
        for pos in body.iter() {
            let pos = *pos + STAGE_SIZE.as_ivec2() / 2;
            term.put_char(pos, BODY_GLYPH);
        }
    }
}

fn eat(
    q_food: Query<(Entity, &Food)>,
    q_snake: Query<(&Body, &GridPos), Changed<GridPos>>,
    mut commands: Commands,
    mut count: ResMut<FoodCount>,
    nom: Res<NomSound>,
    mut tick: ResMut<TickRate>,
) {
    for (body, pos) in &q_snake {
        for (e_food, food) in &q_food {
            if pos.0 == food.pos {
                count.0 += 1;
                commands.entity(e_food).despawn();
                commands.spawn(Grow {
                    turns: count.0,
                    pos: *body.0.back().unwrap(),
                });

                commands.spawn((AudioPlayer::new(nom.0.clone()), PlaybackSettings::DESPAWN));
                let mut dur = tick.duration().as_secs_f32();
                dur = (dur - ACCELERATION).max(MIN_TICK_DELAY);
                tick.set_duration(Duration::from_secs_f32(dur));
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
    ouch: Res<OuchSound>,
) {
    let mut game_over = |entity| {
        commands.entity(entity).despawn();
        q_food.iter().for_each(|e| commands.entity(e).despawn());
        let mut term = q_term.single_mut();
        term.clear();
        term.put_string(
            [0, 0].pivot(Pivot::Center),
            "Game Over!\nPress Space to Restart",
        );
        commands.spawn((
            AudioPlayer::new(ouch.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::new(0.5)),
        ));
    };

    if let Ok((entity, pos, body)) = q_snake.get_single() {
        let min = -STAGE_SIZE.as_ivec2() / 2;
        let max = min + STAGE_SIZE.as_ivec2();
        let bounds = IRect::from_corners(min, max);
        if !bounds.contains(pos.0) {
            game_over(entity);
            return;
        }

        for p in body.0.iter().skip(1) {
            if *p == pos.0 {
                game_over(entity);
                return;
            }
        }
    }
}
