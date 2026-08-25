use core::time::Duration;
use std::io::{IsTerminal as _, Write};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use crate::args::ColorChoice;

use super::{clock, compact_elapsed};
use crate::report::output::color_enabled;

const FRAMES: &[char] = &['⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏', '⠛'];
const TICK: Duration = Duration::from_millis(80);

struct Phase {
    name: String,
    started: Instant,
}

struct State {
    phases: Vec<Phase>,
    command_started: Instant,
    frame: usize,
    drawn: bool,
    suspended: usize,
    worker_running: bool,
    color: bool,
}

impl State {
    fn new() -> Self {
        Self {
            phases: Vec::new(),
            command_started: Instant::now(),
            frame: 0,
            drawn: false,
            suspended: 0,
            worker_running: false,
            color: false,
        }
    }
}

struct Renderer {
    interactive: bool,
    state: Mutex<State>,
}

static RENDERER: OnceLock<Renderer> = OnceLock::new();

pub(super) fn interactive() -> bool {
    renderer().interactive
}

pub(super) fn started(name: &str, choice: ColorChoice) {
    let renderer = renderer();
    let mut state = lock(renderer);
    let mut stderr = std::io::stderr().lock();
    clear(&mut state, &mut stderr);
    state.phases.push(Phase {
        name: name.to_owned(),
        started: Instant::now(),
    });
    state.color = color_enabled(choice);
    draw(&mut state, &mut stderr);
    if !state.worker_running {
        state.worker_running = true;
        drop(state);
        drop(std::thread::spawn(worker));
    }
}

pub(super) fn complete(name: &str, elapsed: Option<Duration>) {
    let renderer = renderer();
    let mut state = lock(renderer);
    let mut stderr = std::io::stderr().lock();
    clear(&mut state, &mut stderr);
    let measured = remove_phase(&mut state, name);
    let duration = elapsed.or(measured).unwrap_or_default();
    let (symbol, verb, color) = if elapsed.is_some() {
        ('✓', "Finished", "32")
    } else {
        ('✗', "Failed", "31")
    };
    let symbol = paint(symbol, color, state.color);
    let _ = writeln!(
        stderr,
        "  {symbol}  {:>7}  {verb} {name}",
        compact_elapsed(duration)
    );
    draw(&mut state, &mut stderr);
}

pub(super) fn suspend<T>(operation: impl FnOnce() -> T) -> T {
    if !interactive() {
        return operation();
    }
    let renderer = renderer();
    {
        let mut state = lock(renderer);
        clear(&mut state, &mut std::io::stderr().lock());
        state.suspended += 1;
    }
    let resume = Resume(renderer);
    let result = operation();
    drop(resume);
    result
}

fn renderer() -> &'static Renderer {
    RENDERER.get_or_init(|| Renderer {
        interactive: std::io::stderr().is_terminal()
            && std::env::var_os("TERM").is_none_or(|term| term != "dumb"),
        state: Mutex::new(State::new()),
    })
}

fn lock(renderer: &Renderer) -> MutexGuard<'_, State> {
    renderer
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn worker() {
    loop {
        std::thread::sleep(TICK);
        let renderer = renderer();
        let mut state = lock(renderer);
        if state.phases.is_empty() {
            state.worker_running = false;
            return;
        }
        state.frame = (state.frame + 1) % FRAMES.len();
        if state.suspended == 0 {
            draw(&mut state, &mut std::io::stderr().lock());
        }
    }
}

fn remove_phase(state: &mut State, name: &str) -> Option<Duration> {
    let index = state.phases.iter().position(|phase| phase.name == name)?;
    Some(state.phases.remove(index).started.elapsed())
}

fn draw(state: &mut State, output: &mut impl Write) {
    if state.phases.is_empty() || state.suspended > 0 {
        return;
    }
    clear(state, output);
    let spinner = paint(FRAMES[state.frame], "36", state.color);
    let names = state
        .phases
        .iter()
        .map(|phase| phase.name.as_str())
        .collect::<Vec<_>>()
        .join(" + ");
    let clock = clock(state.command_started.elapsed());
    let _ = write!(output, "  {spinner}  {clock:>7}  Checking {names}");
    let _ = output.flush();
    state.drawn = true;
}

fn clear(state: &mut State, output: &mut impl Write) {
    if state.drawn {
        let _ = write!(output, "\r\u{1b}[2K");
        state.drawn = false;
    }
}

fn paint(character: char, color: &str, enabled: bool) -> String {
    if enabled {
        format!("\u{1b}[{color}m{character}\u{1b}[0m")
    } else {
        character.to_string()
    }
}

struct Resume(&'static Renderer);

impl Drop for Resume {
    fn drop(&mut self) {
        let mut state = lock(self.0);
        state.suspended = state.suspended.saturating_sub(1);
        draw(&mut state, &mut std::io::stderr().lock());
    }
}
