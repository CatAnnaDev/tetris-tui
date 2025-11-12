use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, ClearType},
};
use rand::Rng;
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

const WIDTH: usize = 10;
const HEIGHT: usize = 20;
const BLOCK: &str = "██";

#[derive(Clone, Copy, PartialEq)]
enum TetrominoType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[derive(Clone, Copy, PartialEq)]
enum PowerUpType {
    Bomb,
    SlowTime,
    Ghost,
    Hammer,
    Random,
}

#[derive(Clone, Copy, PartialEq)]
enum CellType {
    Normal(Color),
    Obstacle,
    PowerUp(PowerUpType),
}

#[derive(Clone)]
struct Tetromino {
    shape: Vec<Vec<bool>>,
    color: Color,
    typ: TetrominoType,
}

impl Tetromino {
    fn new(typ: TetrominoType) -> Self {
        let (shape, color) = match typ {
            TetrominoType::I => (
                vec![
                    vec![false, false, false, false],
                    vec![true, true, true, true],
                    vec![false, false, false, false],
                    vec![false, false, false, false],
                ],
                Color::Cyan,
            ),
            TetrominoType::O => (vec![vec![true, true], vec![true, true]], Color::Yellow),
            TetrominoType::T => (
                vec![
                    vec![false, true, false],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Color::Magenta,
            ),
            TetrominoType::S => (
                vec![
                    vec![false, true, true],
                    vec![true, true, false],
                    vec![false, false, false],
                ],
                Color::Green,
            ),
            TetrominoType::Z => (
                vec![
                    vec![true, true, false],
                    vec![false, true, true],
                    vec![false, false, false],
                ],
                Color::Red,
            ),
            TetrominoType::J => (
                vec![
                    vec![true, false, false],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Color::Blue,
            ),
            TetrominoType::L => (
                vec![
                    vec![false, false, true],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Color::White,
            ),
        };
        Tetromino { shape, color, typ }
    }

    fn rotate(&mut self) {
        let n = self.shape.len();
        let mut rotated = vec![vec![false; n]; n];
        for i in 0..n {
            for j in 0..n {
                rotated[j][n - 1 - i] = self.shape[i][j];
            }
        }
        self.shape = rotated;
    }
}

struct Game {
    board: Vec<Vec<Option<CellType>>>,
    current: Tetromino,
    current_x: i32,
    current_y: i32,
    next: Tetromino,
    score: u32,
    combo: u32,
    game_over: bool,
    ghost_mode: bool,
    ghost_remaining: u32,
    slow_time_active: bool,
    slow_time_end: Option<Instant>,
    hammer_mode: bool,
    last_clear_time: Option<Instant>,
    lines_cleared_total: u32,
}

impl Game {
    fn new() -> Self {
        let mut rng = rand::rng();
        let types = [
            TetrominoType::I,
            TetrominoType::O,
            TetrominoType::T,
            TetrominoType::S,
            TetrominoType::Z,
            TetrominoType::J,
            TetrominoType::L,
        ];

        Game {
            board: vec![vec![None; WIDTH]; HEIGHT],
            current: Tetromino::new(types[rng.random_range(0..7)]),
            current_x: (WIDTH / 2 - 2) as i32,
            current_y: 0,
            next: Tetromino::new(types[rng.random_range(0..7)]),
            score: 0,
            combo: 0,
            game_over: false,
            ghost_mode: false,
            ghost_remaining: 0,
            slow_time_active: false,
            slow_time_end: None,
            hammer_mode: false,
            last_clear_time: None,
            lines_cleared_total: 0,
        }
    }

    fn can_move(&self, dx: i32, dy: i32) -> bool {
        for (i, row) in self.current.shape.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell {
                    let new_x = self.current_x + j as i32 + dx;
                    let new_y = self.current_y + i as i32 + dy;

                    if new_x < 0 || new_x >= WIDTH as i32 || new_y >= HEIGHT as i32 {
                        return false;
                    }

                    if new_y >= 0 {
                        if let Some(cell_type) = &self.board[new_y as usize][new_x as usize] {
                            match cell_type {
                                CellType::Obstacle if !self.ghost_mode => return false,
                                CellType::Normal(_) if !self.ghost_mode => return false,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        if self.can_move(dx, dy) {
            self.current_x += dx;
            self.current_y += dy;
            true
        } else {
            false
        }
    }

    fn rotate_piece(&mut self) {
        let mut rotated = self.current.clone();
        rotated.rotate();
        let old = self.current.clone();
        self.current = rotated;

        if !self.can_move(0, 0) {
            self.current = old;
        } else {
            play_sound(300, 30);
        }
    }

    fn lock_piece(&mut self) {
        for (i, row) in self.current.shape.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell {
                    let x = (self.current_x + j as i32) as usize;
                    let y = (self.current_y + i as i32) as usize;
                    if y < HEIGHT {
                        self.board[y][x] = Some(CellType::Normal(self.current.color));
                    }
                }
            }
        }

        let freq = match self.current.typ {
            TetrominoType::I => 440,
            TetrominoType::O => 494,
            TetrominoType::T => 523,
            TetrominoType::S => 587,
            TetrominoType::Z => 659,
            TetrominoType::J => 698,
            TetrominoType::L => 784,
        };
        play_sound(freq, 50);

        self.collect_power_ups();
        self.clear_lines();
        self.spawn_new_piece();

        if self.ghost_mode && self.ghost_remaining > 0 {
            self.ghost_remaining -= 1;
            if self.ghost_remaining == 0 {
                self.ghost_mode = false;
            }
        }
    }

    fn collect_power_ups(&mut self) {
        let shape = self.current.shape.clone();
        let current_x = self.current_x;
        let current_y = self.current_y;
        let color = self.current.color;

        let mut power_ups_to_activate = Vec::new();

        for (i, row) in shape.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell {
                    let x = (current_x + j as i32) as usize;
                    let y = (current_y + i as i32) as usize;
                    if y < HEIGHT {
                        if let Some(CellType::PowerUp(powerup)) = self.board[y][x] {
                            power_ups_to_activate.push((x, y, powerup));
                        }
                    }
                }
            }
        }

        for (x, y, power_up) in power_ups_to_activate {
            self.activate_power_up(power_up);
            self.board[y][x] = Some(CellType::Normal(color));
        }
    }

    fn activate_power_up(&mut self, powerup: PowerUpType) {
        play_sound(800, 100);

        match powerup {
            PowerUpType::Bomb => {
                let mut cx = 0;
                let mut cy = 0;
                let mut count = 0;
                for (i, row) in self.current.shape.iter().enumerate() {
                    for (j, &cell) in row.iter().enumerate() {
                        if cell {
                            cx += self.current_x + j as i32;
                            cy += self.current_y + i as i32;
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    cx /= count;
                    cy /= count;

                    for dy in -2..=2 {
                        for dx in -2..=2 {
                            let x = (cx + dx) as usize;
                            let y = (cy + dy) as usize;
                            if x < WIDTH && y < HEIGHT {
                                if let Some(CellType::Normal(_)) = self.board[y][x] {
                                    self.board[y][x] = None;
                                    self.score += 10;
                                }
                            }
                        }
                    }
                }
            }
            PowerUpType::SlowTime => {
                self.slow_time_active = true;
                self.slow_time_end = Some(Instant::now() + Duration::from_secs(10));
            }
            PowerUpType::Ghost => {
                self.ghost_mode = true;
                self.ghost_remaining = 3;
            }
            PowerUpType::Hammer => {
                self.hammer_mode = true;
            }
            PowerUpType::Random => {
                let mut rng = rand::rng();
                let powerups = [
                    PowerUpType::Bomb,
                    PowerUpType::SlowTime,
                    PowerUpType::Ghost,
                    PowerUpType::Hammer,
                ];
                self.activate_power_up(powerups[rng.random_range(0..4)]);
            }
        }
    }

    fn clear_lines(&mut self) {
        let mut lines_to_clear = Vec::new();

        for y in 0..HEIGHT {
            let full = self.board[y]
                .iter()
                .all(|cell| matches!(cell, Some(CellType::Normal(_))));
            if full {
                lines_to_clear.push(y);
            }
        }

        if !lines_to_clear.is_empty() {
            for i in 0..lines_to_clear.len() {
                play_sound(800 + (i * 200) as u32, 50);
            }

            let now = Instant::now();
            if let Some(last) = self.last_clear_time {
                if now.duration_since(last) < Duration::from_secs(3) {
                    self.combo += 1;
                } else {
                    self.combo = 0;
                }
            }
            self.last_clear_time = Some(now);

            let lines_cleared = lines_to_clear.len() as u32;
            self.lines_cleared_total += lines_cleared;

            let base_score = match lines_cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            };
            self.score += base_score * (1 + self.combo);

            for line in lines_to_clear.iter().rev() {
                self.board.remove(*line);
                self.board.insert(0, vec![None; WIDTH]);
            }

            self.apply_gravity();

            let mut rng = rand::rng();
            if self.lines_cleared_total % 5 == 0 && rng.random_bool(0.3) {
                self.spawn_obstacle();
            }
            if rng.random_bool(0.4) {
                self.spawn_power_up();
            }
        } else {
            self.combo = 0;
        }
    }

    fn apply_gravity(&mut self) {
        for _ in 0..HEIGHT {
            for y in (0..HEIGHT - 1).rev() {
                for x in 0..WIDTH {
                    if self.board[y][x].is_some() && self.board[y + 1][x].is_none() {
                        self.board[y + 1][x] = self.board[y][x];
                        self.board[y][x] = None;
                    }
                }
            }
        }
    }

    fn spawn_obstacle(&mut self) {
        let mut rng = rand::rng();
        let x = rng.random_range(0..WIDTH);
        let y = HEIGHT - 1;

        if self.board[y][x].is_none() {
            self.board[y][x] = Some(CellType::Obstacle);
        }
    }

    fn spawn_power_up(&mut self) {
        let mut rng = rand::rng();
        let x = rng.random_range(0..WIDTH);
        let y = HEIGHT - 1;

        if self.board[y][x].is_none() {
            let powerups = [
                PowerUpType::Bomb,
                PowerUpType::SlowTime,
                PowerUpType::Ghost,
                PowerUpType::Hammer,
                PowerUpType::Random,
            ];
            self.board[y][x] = Some(CellType::PowerUp(powerups[rng.random_range(0..5)]));
        }
    }

    fn spawn_new_piece(&mut self) {
        self.current = self.next.clone();
        self.current_x = (WIDTH / 2 - 2) as i32;
        self.current_y = 0;

        let mut rng = rand::rng();
        let types = [
            TetrominoType::I,
            TetrominoType::O,
            TetrominoType::T,
            TetrominoType::S,
            TetrominoType::Z,
            TetrominoType::J,
            TetrominoType::L,
        ];
        self.next = Tetromino::new(types[rng.random_range(0..7)]);

        if !self.can_move(0, 0) {
            self.game_over = true;
            play_sound(200, 100);
            play_sound(150, 100);
            play_sound(100, 200);
        }
    }

    fn drop_piece(&mut self) {
        while self.move_piece(0, 1) {}
        play_sound(600, 80);
        self.lock_piece();
    }

    fn use_hammer(&mut self, line: usize) {
        if line < HEIGHT {
            self.board.remove(line);
            self.board.insert(0, vec![None; WIDTH]);
            self.hammer_mode = false;
            self.score += 50;
            play_sound(400, 100);
            self.apply_gravity();
        }
    }

    fn get_fall_speed(&self) -> Duration {
        let base_speed = 500;
        let speed = if self.slow_time_active {
            base_speed * 2
        } else {
            base_speed
        };
        Duration::from_millis(speed)
    }
}

fn play_sound(_frequency: u32, duration_ms: u64) {
    print!("\x07");
    io::stdout().flush().unwrap();
    std::thread::sleep(Duration::from_millis(duration_ms / 10));
}

fn draw(stdout: &mut io::Stdout, game: &Game) -> io::Result<()> {
    queue!(stdout, cursor::MoveTo(0, 0))?;

    queue!(
        stdout,
        SetForegroundColor(Color::White),
        Print("╔"),
        Print("═".repeat(WIDTH * 2)),
        Print("╗\n\r")
    )?;
    queue!(
        stdout,
        Print("║"),
        SetForegroundColor(Color::Red),
        Print(format!(
            "{:^width$}",
            "⚡ TETRIS CHAOS ⚡",
            width = WIDTH * 2
        )),
        SetForegroundColor(Color::White),
        Print("║\n\r")
    )?;
    queue!(
        stdout,
        Print("╠"),
        Print("═".repeat(WIDTH * 2)),
        Print("╣\n\r")
    )?;

    for y in 0..HEIGHT {
        queue!(stdout, SetForegroundColor(Color::White), Print("║"))?;

        if game.hammer_mode {
            queue!(stdout, SetForegroundColor(Color::DarkYellow), Print(""))?;
        }

        for x in 0..WIDTH {
            let mut drawn = false;

            for (i, row) in game.current.shape.iter().enumerate() {
                for (j, &cell) in row.iter().enumerate() {
                    if cell {
                        let px = game.current_x + j as i32;
                        let py = game.current_y + i as i32;
                        if px == x as i32 && py == y as i32 {
                            let color = if game.ghost_mode {
                                Color::DarkCyan
                            } else {
                                game.current.color
                            };
                            queue!(stdout, SetForegroundColor(color), Print(BLOCK))?;
                            drawn = true;
                        }
                    }
                }
            }

            if !drawn {
                match &game.board[y][x] {
                    Some(CellType::Normal(color)) => {
                        queue!(stdout, SetForegroundColor(*color), Print(BLOCK))?;
                    }
                    Some(CellType::Obstacle) => {
                        queue!(stdout, SetForegroundColor(Color::DarkGrey), Print("▓▓"))?;
                    }
                    Some(CellType::PowerUp(powerup)) => {
                        let (symbol, color) = match powerup {
                            PowerUpType::Bomb => ("💣", Color::Red),
                            PowerUpType::SlowTime => ("⏰", Color::Cyan),
                            PowerUpType::Ghost => ("👻", Color::White),
                            PowerUpType::Hammer => ("🔨", Color::Yellow),
                            PowerUpType::Random => ("🎲", Color::Magenta),
                        };
                        queue!(stdout, SetForegroundColor(color), Print(symbol))?;
                    }
                    None => {
                        queue!(stdout, Print("  "))?;
                    }
                }
            }
        }

        queue!(stdout, SetForegroundColor(Color::White), Print("║"))?;

        match y {
            1 => queue!(
                stdout,
                Print("  Score: "),
                SetForegroundColor(Color::Yellow),
                Print(format!("{}", game.score))
            )?,
            2 => {
                if game.combo > 0 {
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Red),
                        Print(format!("  COMBO x{}", game.combo + 1))
                    )?;
                }
            }
            4 => queue!(
                stdout,
                SetForegroundColor(Color::White),
                Print("  Suivant:")
            )?,
            5..=8 => {
                let ny = y - 5;
                queue!(stdout, Print("  "))?;
                for j in 0..4 {
                    if ny < game.next.shape.len()
                        && j < game.next.shape[ny].len()
                        && game.next.shape[ny][j]
                    {
                        queue!(stdout, SetForegroundColor(game.next.color), Print(BLOCK))?;
                    } else {
                        queue!(stdout, Print("  "))?;
                    }
                }
            }
            10 => queue!(
                stdout,
                SetForegroundColor(Color::Cyan),
                Print("  Power-ups:")
            )?,
            11 => {
                if game.ghost_mode {
                    queue!(
                        stdout,
                        SetForegroundColor(Color::White),
                        Print(format!("  👻 Ghost x{}", game.ghost_remaining))
                    )?;
                }
            }
            12 => {
                if game.slow_time_active {
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Cyan),
                        Print("  ⏰ Slow Time")
                    )?;
                }
            }
            13 => {
                if game.hammer_mode {
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Yellow),
                        Print("  🔨 Hammer: 1-9")
                    )?;
                }
            }
            15 => queue!(
                stdout,
                SetForegroundColor(Color::White),
                Print("  Contrôles:")
            )?,
            16 => queue!(stdout, Print("  ←→↑↓ Jouer"))?,
            17 => queue!(stdout, Print("  Space: Drop"))?,
            18 => queue!(stdout, Print("  Q: Quitter"))?,
            _ => {}
        }

        queue!(stdout, Print("\n\r"))?;
    }

    queue!(
        stdout,
        SetForegroundColor(Color::White),
        Print("╚"),
        Print("═".repeat(WIDTH * 2)),
        Print("╝\n\r")
    )?;

    if game.game_over {
        queue!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("\n\r💀 GAME OVER 💀 Score: "),
            Print(format!("{}", game.score)),
            Print("\n\r")
        )?;
    }

    stdout.flush()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::Hide)?;

    let mut game = Game::new();
    let mut last_fall = Instant::now();

    loop {
        draw(&mut stdout, &game)?;

        if game.game_over {
            terminal::disable_raw_mode()?;
            execute!(stdout, cursor::Show)?;
            break;
        }

        if game.slow_time_active {
            if let Some(end_time) = game.slow_time_end {
                if Instant::now() >= end_time {
                    game.slow_time_active = false;
                    game.slow_time_end = None;
                }
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Left => {
                        game.move_piece(-1, 0);
                    }
                    KeyCode::Right => {
                        game.move_piece(1, 0);
                    }
                    KeyCode::Down => {
                        if !game.move_piece(0, 1) {
                            game.lock_piece();
                        }
                    }
                    KeyCode::Up => {
                        game.rotate_piece();
                    }
                    KeyCode::Char(' ') => {
                        game.drop_piece();
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,

                    KeyCode::Char(c) if game.hammer_mode && c.is_digit(10) => {
                        if let Some(digit) = c.to_digit(10) {
                            if digit > 0 && digit <= HEIGHT as u32 {
                                game.use_hammer(HEIGHT - digit as usize);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let fall_speed = game.get_fall_speed();
        if last_fall.elapsed() >= fall_speed {
            if !game.move_piece(0, 1) {
                game.lock_piece();
            }
            last_fall = Instant::now();
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;
    Ok(())
}
