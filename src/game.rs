// SPDX-License-Identifier: GPL-3.0-or-later
//
// Gameplay behavior is a clean Rust implementation of GNOME Mines.

use rand::{Rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const NEIGHBOURS: [(i32, i32); 8] = [
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mark {
    #[default]
    None,
    Flag,
    Maybe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub mine: bool,
    pub revealed: bool,
    pub mark: Mark,
    pub adjacent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    InProgress,
    Won,
    Lost,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub cells: Vec<Cell>,
    pub placed: bool,
    pub outcome: Outcome,
    pub revealed_count: usize,
    pub flag_count: usize,
    pub exploded_cell: Option<usize>,
}

impl Board {
    pub fn new(width: usize, height: usize, mine_count: usize) -> Self {
        assert!((4..=100).contains(&width));
        assert!((4..=100).contains(&height));
        assert!((1..=width * height - 10).contains(&mine_count));
        Self {
            width,
            height,
            mine_count,
            cells: vec![Cell::default(); width * height],
            placed: false,
            outcome: Outcome::InProgress,
            revealed_count: 0,
            flag_count: 0,
            exploded_cell: None,
        }
    }

    #[inline]
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    #[inline]
    pub fn coords(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }

    #[inline]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
    }

    pub fn neighbours(&self, x: usize, y: usize) -> impl Iterator<Item = usize> + '_ {
        NEIGHBOURS.into_iter().filter_map(move |(dx, dy)| {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            self.contains(nx, ny)
                .then(|| self.index(nx as usize, ny as usize))
        })
    }

    pub fn reveal<R: Rng + ?Sized>(
        &mut self,
        x: usize,
        y: usize,
        rng: &mut R,
        auto_clear_on_flags: bool,
        use_autoflag: bool,
    ) -> bool {
        if self.outcome != Outcome::InProgress {
            return false;
        }
        if !self.placed {
            self.place_mines(x, y, rng);
        }
        let index = self.index(x, y);
        if self.cells[index].revealed || self.cells[index].mark == Mark::Flag {
            return false;
        }
        self.flood_reveal(index, auto_clear_on_flags, use_autoflag);
        self.check_win();
        true
    }

    pub fn chord(
        &mut self,
        x: usize,
        y: usize,
        auto_clear_on_flags: bool,
        use_autoflag: bool,
    ) -> bool {
        if self.outcome != Outcome::InProgress {
            return false;
        }
        let index = self.index(x, y);
        if !self.cells[index].revealed {
            return false;
        }
        let changed = self.check_around(index, true, auto_clear_on_flags, use_autoflag);
        self.check_win();
        changed
    }

    pub fn cycle_mark(
        &mut self,
        x: usize,
        y: usize,
        use_question_marks: bool,
        auto_clear_on_flags: bool,
        use_autoflag: bool,
    ) -> bool {
        if self.outcome != Outcome::InProgress {
            return false;
        }
        let index = self.index(x, y);
        if self.cells[index].revealed {
            return false;
        }

        let next = match self.cells[index].mark {
            Mark::None if self.flag_count >= self.mine_count && use_question_marks => Mark::Maybe,
            Mark::None => Mark::Flag,
            Mark::Flag if use_question_marks => Mark::Maybe,
            Mark::Flag => Mark::None,
            Mark::Maybe => Mark::None,
        };
        self.set_mark(index, next);

        if next == Mark::Flag && auto_clear_on_flags {
            let (cx, cy) = self.coords(index);
            let nearby: Vec<_> = self.neighbours(cx, cy).collect();
            for neighbour in nearby {
                if self.cells[neighbour].revealed {
                    self.check_around(neighbour, true, auto_clear_on_flags, use_autoflag);
                    if self.outcome == Outcome::Lost {
                        break;
                    }
                }
            }
            self.check_win();
        }
        true
    }

    fn set_mark(&mut self, index: usize, mark: Mark) {
        if self.cells[index].revealed || self.cells[index].mark == mark {
            return;
        }
        if self.cells[index].mark == Mark::Flag {
            self.flag_count -= 1;
        }
        if mark == Mark::Flag {
            self.flag_count += 1;
        }
        self.cells[index].mark = mark;
    }

    fn place_mines<R: Rng + ?Sized>(&mut self, safe_x: usize, safe_y: usize, rng: &mut R) {
        let mut candidates: Vec<usize> = (0..self.cells.len())
            .filter(|&index| {
                let (x, y) = self.coords(index);
                (x as i32 - safe_x as i32).abs() > 1 || (y as i32 - safe_y as i32).abs() > 1
            })
            .collect();
        candidates.shuffle(rng);

        for &index in candidates.iter().take(self.mine_count) {
            self.cells[index].mine = true;
        }

        for index in 0..self.cells.len() {
            let (x, y) = self.coords(index);
            self.cells[index].adjacent = self
                .neighbours(x, y)
                .filter(|&nearby| self.cells[nearby].mine)
                .count() as u8;
        }
        self.placed = true;
    }

    fn flood_reveal(
        &mut self,
        start: usize,
        auto_clear_on_flags: bool,
        use_autoflag: bool,
    ) -> bool {
        let mut changed = false;
        let mut queue = VecDeque::from([start]);

        while let Some(index) = queue.pop_front() {
            if self.cells[index].revealed
                || (auto_clear_on_flags && self.cells[index].mark == Mark::Flag)
            {
                continue;
            }

            if self.cells[index].mark == Mark::Flag {
                self.flag_count -= 1;
            }
            self.cells[index].mark = Mark::None;
            self.cells[index].revealed = true;
            self.revealed_count += 1;
            changed = true;

            if self.cells[index].mine {
                self.outcome = Outcome::Lost;
                self.exploded_cell = Some(index);
                return true;
            }

            if self.cells[index].adjacent == 0 {
                let (x, y) = self.coords(index);
                queue.extend(self.neighbours(x, y));
            } else if auto_clear_on_flags || use_autoflag {
                self.check_around(
                    index,
                    auto_clear_on_flags,
                    auto_clear_on_flags,
                    use_autoflag,
                );
                if self.outcome == Outcome::Lost {
                    return true;
                }
            }
        }
        changed
    }

    fn check_around(
        &mut self,
        index: usize,
        compare_with_flags: bool,
        auto_clear_on_flags: bool,
        use_autoflag: bool,
    ) -> bool {
        if !self.cells[index].revealed || self.cells[index].mark == Mark::Flag {
            return false;
        }

        let (x, y) = self.coords(index);
        let nearby: Vec<_> = self.neighbours(x, y).collect();
        let flags = nearby
            .iter()
            .filter(|&&i| self.cells[i].mark == Mark::Flag)
            .count() as u8;
        let unknown = nearby.iter().filter(|&&i| !self.cells[i].revealed).count() as u8;
        let adjacent = self.cells[index].adjacent;

        if compare_with_flags && adjacent == flags {
            let mut changed = false;
            for neighbour in nearby {
                if self.cells[neighbour].mark != Mark::Flag {
                    changed |= self.flood_reveal(neighbour, auto_clear_on_flags, use_autoflag);
                    if self.outcome == Outcome::Lost {
                        break;
                    }
                }
            }
            changed
        } else if use_autoflag && unknown == adjacent {
            let mut changed = false;
            for neighbour in nearby {
                if !self.cells[neighbour].revealed {
                    changed |= self.cells[neighbour].mark != Mark::Flag;
                    self.set_mark(neighbour, Mark::Flag);
                }
            }
            changed
        } else {
            false
        }
    }

    /// Development helper used by the debug cheat key: reveal every safe cell and
    /// evaluate the win condition so the victory flow can be exercised quickly.
    #[cfg(debug_assertions)]
    pub fn debug_reveal_all(&mut self) {
        if !self.placed {
            let mut rng = rand::rng();
            self.place_mines(0, 0, &mut rng);
        }
        for cell in self.cells.iter_mut() {
            if !cell.mine {
                cell.revealed = true;
                cell.mark = Mark::None;
            }
        }
        self.revealed_count = self.cells.iter().filter(|cell| cell.revealed).count();
        self.flag_count = self
            .cells
            .iter()
            .filter(|cell| cell.mark == Mark::Flag)
            .count();
        self.check_win();
    }

    fn check_win(&mut self) {
        if self.outcome == Outcome::InProgress
            && self.revealed_count == self.width * self.height - self.mine_count
        {
            self.outcome = Outcome::Won;
            for index in 0..self.cells.len() {
                if self.cells[index].mine {
                    self.set_mark(index, Mark::Flag);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameClock {
    accumulated: Duration,
    started_at: Option<Instant>,
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            accumulated: Duration::ZERO,
            started_at: None,
        }
    }
}

impl GameClock {
    pub fn start(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(Instant::now());
        }
    }

    pub fn pause(&mut self) {
        if let Some(started) = self.started_at.take() {
            self.accumulated += started.elapsed();
        }
    }

    pub fn resume(&mut self) {
        if self.started_at.is_none() && self.accumulated > Duration::ZERO {
            self.started_at = Some(Instant::now());
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.accumulated + self.started_at.map_or(Duration::ZERO, |at| at.elapsed())
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some() || self.accumulated > Duration::ZERO
    }

    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn board() -> Board {
        Board::new(8, 8, 10)
    }

    #[test]
    fn first_reveal_and_neighbours_are_safe() {
        let mut board = board();
        let mut rng = StdRng::seed_from_u64(42);
        assert!(board.reveal(3, 3, &mut rng, false, false));
        for y in 2..=4 {
            for x in 2..=4 {
                assert!(!board.cells[board.index(x, y)].mine);
            }
        }
        assert_eq!(board.cells.iter().filter(|cell| cell.mine).count(), 10);
    }

    #[test]
    fn mark_cycle_matches_question_flag_setting() {
        let mut board = board();
        assert!(board.cycle_mark(0, 0, true, false, false));
        assert_eq!(board.cells[0].mark, Mark::Flag);
        assert!(board.cycle_mark(0, 0, true, false, false));
        assert_eq!(board.cells[0].mark, Mark::Maybe);
        assert!(board.cycle_mark(0, 0, true, false, false));
        assert_eq!(board.cells[0].mark, Mark::None);

        assert!(board.cycle_mark(0, 0, false, false, false));
        assert!(board.cycle_mark(0, 0, false, false, false));
        assert_eq!(board.cells[0].mark, Mark::None);
    }

    #[test]
    fn cannot_place_more_flags_than_mines_when_question_marks_are_on() {
        let mut board = board();
        for x in 0..10 {
            board.cycle_mark(x % 8, x / 8, true, false, false);
        }
        board.cycle_mark(2, 2, true, false, false);
        assert_eq!(board.cells[board.index(2, 2)].mark, Mark::Maybe);
        assert_eq!(board.flag_count, 10);
    }

    #[test]
    fn chord_with_incorrect_flags_can_explode() {
        let mut board = Board::new(4, 4, 1);
        board.placed = true;
        let mine = board.index(0, 0);
        board.cells[mine].mine = true;
        for index in [board.index(0, 1), board.index(1, 0), board.index(1, 1)] {
            board.cells[index].adjacent = 1;
        }
        let center = board.index(1, 1);
        board.cells[center].revealed = true;
        board.revealed_count = 1;
        board.cycle_mark(2, 1, true, false, false);
        board.chord(1, 1, false, false);
        assert_eq!(board.outcome, Outcome::Lost);
    }

    #[test]
    fn revealing_every_safe_cell_wins_and_marks_the_mine() {
        let mut board = Board::new(4, 4, 1);
        board.placed = true;
        let mine = board.index(0, 0);
        board.cells[mine].mine = true;
        for index in [board.index(0, 1), board.index(1, 0), board.index(1, 1)] {
            board.cells[index].adjacent = 1;
        }
        let mut rng = StdRng::seed_from_u64(7);
        for y in 0..4 {
            for x in 0..4 {
                if (x, y) != (0, 0) {
                    board.reveal(x, y, &mut rng, false, false);
                }
            }
        }
        assert_eq!(board.outcome, Outcome::Won);
        assert_eq!(board.cells[mine].mark, Mark::Flag);
        assert_eq!(board.flag_count, 1);
    }

    #[test]
    fn placing_last_correct_flag_can_auto_clear_and_win() {
        let mut board = Board::new(4, 4, 1);
        board.placed = true;
        let mine = board.index(0, 0);
        board.cells[mine].mine = true;
        for index in [board.index(0, 1), board.index(1, 0), board.index(1, 1)] {
            board.cells[index].adjacent = 1;
        }
        let center = board.index(1, 1);
        board.cells[center].revealed = true;
        board.revealed_count = 1;

        board.cycle_mark(0, 0, true, true, false);

        assert_eq!(board.outcome, Outcome::Won);
        assert_eq!(board.revealed_count, 15);
    }

    #[test]
    fn autoflag_marks_the_only_hidden_neighbour_on_chord() {
        let mut board = Board::new(4, 4, 1);
        board.placed = true;
        let mine = board.index(0, 0);
        board.cells[mine].mine = true;
        let center = board.index(1, 1);
        board.cells[center].adjacent = 1;
        let around: Vec<_> = board.neighbours(1, 1).collect();
        for index in around {
            if index != mine {
                board.cells[index].revealed = true;
                board.revealed_count += 1;
            }
        }
        board.cells[center].revealed = true;
        board.revealed_count += 1;

        board.chord(1, 1, false, true);

        assert_eq!(board.cells[mine].mark, Mark::Flag);
        assert_eq!(board.flag_count, 1);
    }
}
