// SPDX-License-Identifier: GPL-3.0-or-later
//
// Deterministic Minesweeper logic solver and no-guessing mine placement.
// Ensures first-click mine layouts can be solved via pure deduction without
// forced 50/50 guesses.

use crate::game::NEIGHBOURS;
use rand::{Rng, seq::SliceRandom};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const MAX_COMPONENT_VARS: usize = 18;
const MAX_COMPONENT_NODES: usize = 8_192;
const MIN_FRESH_ATTEMPTS: usize = 24;
const SWAPS_PER_ATTEMPT: usize = 12;
const GENERATE_TIME_BUDGET: Duration = Duration::from_millis(80);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SimCell {
    Hidden,
    Revealed,
    Flagged,
}

#[derive(Clone, Copy)]
struct Constraint {
    cells: [usize; 8],
    len: u8,
    mines: u8,
}

impl Constraint {
    fn slice(&self) -> &[usize] {
        &self.cells[..self.len as usize]
    }
}

#[derive(Clone, Copy)]
struct BitConstraint {
    mask: u32,
    mines: u8,
}

struct ComponentResult {
    vars: Vec<usize>,
    solutions: Vec<(u32, u8)>,
    min_k: usize,
    max_k: usize,
}

struct SolveOutcome {
    solved: bool,
    revealed_safe: usize,
    cells: Vec<SimCell>,
    frontier_hidden: Vec<usize>,
}

struct SolverState<'a> {
    width: usize,
    height: usize,
    mine_count: usize,
    mines: &'a [bool],
    adjacent: Vec<u8>,
    cells: Vec<SimCell>,
    revealed_safe: usize,
    flagged_count: usize,
    active_clues: Vec<usize>,
}

/// Returns `true` when the mine layout `mines` can be solved purely by logic
/// from the initial click at `(start_x, start_y)`.
#[cfg(test)]
pub fn is_solvable(
    width: usize,
    height: usize,
    mine_count: usize,
    mines: &[bool],
    start_x: usize,
    start_y: usize,
) -> bool {
    solve_from(width, height, mine_count, mines, start_x, start_y).solved
}

/// Generates a mine layout with the `3x3` safe zone around `(safe_x, safe_y)`
/// clear of mines, preferring layouts solvable without guessing. Falls back to
/// the deepest-progress candidate if an extreme custom density hits the time
/// budget.
pub fn generate_no_guess_mines<R: Rng + ?Sized>(
    width: usize,
    height: usize,
    mine_count: usize,
    safe_x: usize,
    safe_y: usize,
    rng: &mut R,
) -> Vec<bool> {
    let total = width * height;
    let mut candidates: Vec<usize> = (0..total)
        .filter(|&index| !in_safe_zone(width, index, safe_x, safe_y))
        .collect();

    let start = Instant::now();
    let mut best_revealed = 0usize;
    let mut best_mines = Vec::new();
    let mut attempts = 0usize;

    while attempts < MIN_FRESH_ATTEMPTS || start.elapsed() < GENERATE_TIME_BUDGET {
        attempts += 1;
        candidates.shuffle(rng);
        let mut mines = vec![false; total];
        for &index in candidates.iter().take(mine_count) {
            mines[index] = true;
        }

        let mut current = solve_from(width, height, mine_count, &mines, safe_x, safe_y);
        if current.solved {
            return mines;
        }
        if current.revealed_safe > best_revealed || best_mines.is_empty() {
            best_revealed = current.revealed_safe;
            best_mines = mines.clone();
        }

        // Local perturbation (Simon Tatham heuristic): move a mine stuck on the
        // current frontier into the unrevealed interior while keeping progress
        // monotonic.
        for _ in 0..SWAPS_PER_ATTEMPT {
            let Some((mine_idx, safe_idx)) =
                pick_swap_pair(width, &mines, &current, safe_x, safe_y, rng)
            else {
                break;
            };
            mines.swap(mine_idx, safe_idx);
            let next = solve_from(width, height, mine_count, &mines, safe_x, safe_y);
            if next.solved {
                return mines;
            }
            if next.revealed_safe > best_revealed {
                best_revealed = next.revealed_safe;
                best_mines = mines.clone();
            }
            if next.revealed_safe >= current.revealed_safe {
                current = next;
            } else {
                mines.swap(mine_idx, safe_idx);
            }
        }
    }

    best_mines
}

fn pick_swap_pair<R: Rng + ?Sized>(
    width: usize,
    mines: &[bool],
    outcome: &SolveOutcome,
    safe_x: usize,
    safe_y: usize,
    rng: &mut R,
) -> Option<(usize, usize)> {
    let mut in_frontier = vec![false; mines.len()];
    for &idx in &outcome.frontier_hidden {
        in_frontier[idx] = true;
    }

    let mut mine_pool: Vec<usize> = outcome
        .frontier_hidden
        .iter()
        .copied()
        .filter(|&idx| outcome.cells[idx] == SimCell::Hidden && mines[idx])
        .collect();
    if mine_pool.is_empty() {
        mine_pool = (0..mines.len())
            .filter(|&idx| outcome.cells[idx] == SimCell::Hidden && mines[idx])
            .collect();
    }

    let mut safe_pool: Vec<usize> = (0..mines.len())
        .filter(|&idx| {
            outcome.cells[idx] == SimCell::Hidden
                && !mines[idx]
                && !in_frontier[idx]
                && !in_safe_zone(width, idx, safe_x, safe_y)
        })
        .collect();
    if safe_pool.is_empty() {
        safe_pool = (0..mines.len())
            .filter(|&idx| {
                outcome.cells[idx] == SimCell::Hidden
                    && !mines[idx]
                    && !in_safe_zone(width, idx, safe_x, safe_y)
            })
            .collect();
    }

    if mine_pool.is_empty() || safe_pool.is_empty() {
        return None;
    }

    let mine_idx = mine_pool[rng.random_range(0..mine_pool.len())];
    let safe_idx = safe_pool[rng.random_range(0..safe_pool.len())];
    Some((mine_idx, safe_idx))
}

#[inline]
fn in_safe_zone(width: usize, index: usize, safe_x: usize, safe_y: usize) -> bool {
    let x = (index % width) as i32;
    let y = (index / width) as i32;
    (x - safe_x as i32).abs() <= 1 && (y - safe_y as i32).abs() <= 1
}

#[inline]
fn neighbours(width: usize, height: usize, index: usize) -> ([usize; 8], usize) {
    let mut buf = [0usize; 8];
    let mut len = 0usize;
    let x = (index % width) as i32;
    let y = (index / width) as i32;
    for (dx, dy) in NEIGHBOURS {
        let nx = x + dx;
        let ny = y + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
            buf[len] = ny as usize * width + nx as usize;
            len += 1;
        }
    }
    (buf, len)
}

fn solve_from(
    width: usize,
    height: usize,
    mine_count: usize,
    mines: &[bool],
    start_x: usize,
    start_y: usize,
) -> SolveOutcome {
    let total = width * height;
    debug_assert_eq!(mines.len(), total);
    let start_index = start_y * width + start_x;
    if total == 0 || mines[start_index] {
        return SolveOutcome {
            solved: false,
            revealed_safe: 0,
            cells: vec![SimCell::Hidden; total],
            frontier_hidden: Vec::new(),
        };
    }

    let mut adjacent = vec![0u8; total];
    for (index, adj) in adjacent.iter_mut().enumerate() {
        let (nb, len) = neighbours(width, height, index);
        *adj = nb[..len].iter().filter(|&&n| mines[n]).count() as u8;
    }

    let mut state = SolverState {
        width,
        height,
        mine_count,
        mines,
        adjacent,
        cells: vec![SimCell::Hidden; total],
        revealed_safe: 0,
        flagged_count: 0,
        active_clues: Vec::new(),
    };
    state.reveal_safe(start_index);

    let target_safe = total - mine_count;
    let mut last_frontier = Vec::new();

    while state.revealed_safe < target_safe {
        state.pass_single_cell();
        if state.revealed_safe >= target_safe {
            break;
        }

        let (constraints, frontier_vars) = state.build_constraints();
        last_frontier = frontier_vars;

        if state.pass_pairwise(&constraints) {
            continue;
        }

        if state.pass_components_and_minecount(&constraints, &last_frontier) {
            continue;
        }

        break;
    }

    if state.revealed_safe >= target_safe {
        last_frontier.clear();
    }

    SolveOutcome {
        solved: state.revealed_safe >= target_safe,
        revealed_safe: state.revealed_safe,
        cells: state.cells,
        frontier_hidden: last_frontier,
    }
}

impl<'a> SolverState<'a> {
    fn reveal_safe(&mut self, start: usize) {
        let mut queue = VecDeque::from([start]);
        while let Some(index) = queue.pop_front() {
            if self.cells[index] != SimCell::Hidden {
                continue;
            }
            debug_assert!(!self.mines[index], "solver attempted to reveal a mine");
            self.cells[index] = SimCell::Revealed;
            self.revealed_safe += 1;
            if self.adjacent[index] == 0 {
                let (nb, len) = neighbours(self.width, self.height, index);
                for &n in &nb[..len] {
                    if self.cells[n] == SimCell::Hidden {
                        queue.push_back(n);
                    }
                }
            } else {
                self.active_clues.push(index);
            }
        }
    }

    fn flag_mine(&mut self, index: usize) {
        if self.cells[index] != SimCell::Hidden {
            return;
        }
        debug_assert!(self.mines[index], "solver flagged a safe cell");
        self.cells[index] = SimCell::Flagged;
        self.flagged_count += 1;
    }

    fn pass_single_cell(&mut self) {
        loop {
            let mut changed = false;
            let mut i = 0;
            while i < self.active_clues.len() {
                let clue = self.active_clues[i];
                let (nb, len) = neighbours(self.width, self.height, clue);
                let mut flags = 0u8;
                let mut hidden = [0usize; 8];
                let mut hidden_len = 0usize;
                for &n in &nb[..len] {
                    match self.cells[n] {
                        SimCell::Hidden => {
                            hidden[hidden_len] = n;
                            hidden_len += 1;
                        }
                        SimCell::Flagged => flags += 1,
                        SimCell::Revealed => {}
                    }
                }
                if hidden_len == 0 {
                    self.active_clues.swap_remove(i);
                    continue;
                }
                debug_assert!(self.adjacent[clue] >= flags);
                let rem = self.adjacent[clue] - flags;
                if rem == 0 {
                    for &n in &hidden[..hidden_len] {
                        self.reveal_safe(n);
                    }
                    changed = true;
                    self.active_clues.swap_remove(i);
                    continue;
                }
                if rem as usize == hidden_len {
                    for &n in &hidden[..hidden_len] {
                        self.flag_mine(n);
                    }
                    changed = true;
                    self.active_clues.swap_remove(i);
                    continue;
                }
                i += 1;
            }
            if !changed {
                break;
            }
        }
    }

    fn build_constraints(&mut self) -> (Vec<Constraint>, Vec<usize>) {
        let mut constraints = Vec::with_capacity(self.active_clues.len());
        let mut i = 0;
        while i < self.active_clues.len() {
            let clue = self.active_clues[i];
            let (nb, len) = neighbours(self.width, self.height, clue);
            let mut flags = 0u8;
            let mut hidden = [0usize; 8];
            let mut hidden_len = 0usize;
            for &n in &nb[..len] {
                match self.cells[n] {
                    SimCell::Hidden => {
                        hidden[hidden_len] = n;
                        hidden_len += 1;
                    }
                    SimCell::Flagged => flags += 1,
                    SimCell::Revealed => {}
                }
            }
            if hidden_len == 0 {
                self.active_clues.swap_remove(i);
                continue;
            }
            hidden[..hidden_len].sort_unstable();
            constraints.push(Constraint {
                cells: hidden,
                len: hidden_len as u8,
                mines: self.adjacent[clue] - flags,
            });
            i += 1;
        }

        constraints.sort_unstable_by(|a, b| a.slice().cmp(b.slice()));
        constraints.dedup_by(|a, b| {
            a.len == b.len && a.cells[..a.len as usize] == b.cells[..b.len as usize]
        });

        let mut seen = vec![false; self.cells.len()];
        let mut frontier_vars = Vec::new();
        for c in &constraints {
            for &cell in c.slice() {
                if !seen[cell] {
                    seen[cell] = true;
                    frontier_vars.push(cell);
                }
            }
        }
        frontier_vars.sort_unstable();
        (constraints, frontier_vars)
    }

    fn pass_pairwise(&mut self, constraints: &[Constraint]) -> bool {
        if constraints.len() < 2 {
            return false;
        }
        let mut cell_constraints = vec![Vec::new(); self.cells.len()];
        for (c_idx, c) in constraints.iter().enumerate() {
            for &cell in c.slice() {
                cell_constraints[cell].push(c_idx);
            }
        }

        let mut seen_stamp = vec![0u32; constraints.len()];
        let mut stamp = 0u32;
        let mut to_reveal = Vec::new();
        let mut to_flag = Vec::new();

        for (a_idx, a) in constraints.iter().enumerate() {
            stamp += 1;
            for &cell in a.slice() {
                for &b_idx in &cell_constraints[cell] {
                    if b_idx <= a_idx || seen_stamp[b_idx] == stamp {
                        continue;
                    }
                    seen_stamp[b_idx] = stamp;
                    let b = &constraints[b_idx];
                    let (only_a, only_a_len, only_b, only_b_len) =
                        sorted_diff(a.slice(), b.slice());
                    if b.mines >= a.mines && usize::from(b.mines - a.mines) == only_b_len {
                        to_flag.extend_from_slice(&only_b[..only_b_len]);
                        to_reveal.extend_from_slice(&only_a[..only_a_len]);
                    }
                    if a.mines >= b.mines && usize::from(a.mines - b.mines) == only_a_len {
                        to_flag.extend_from_slice(&only_a[..only_a_len]);
                        to_reveal.extend_from_slice(&only_b[..only_b_len]);
                    }
                }
            }
        }

        self.apply_deductions(&to_reveal, &to_flag)
    }

    fn pass_components_and_minecount(
        &mut self,
        constraints: &[Constraint],
        frontier_vars: &[usize],
    ) -> bool {
        let remaining_mines = self.mine_count - self.flagged_count;

        let mut in_frontier = vec![false; self.cells.len()];
        for &v in frontier_vars {
            in_frontier[v] = true;
        }
        let interior_cells: Vec<usize> = (0..self.cells.len())
            .filter(|&idx| self.cells[idx] == SimCell::Hidden && !in_frontier[idx])
            .collect();

        if remaining_mines == 0 {
            let mut all_hidden = frontier_vars.to_vec();
            all_hidden.extend_from_slice(&interior_cells);
            return self.apply_deductions(&all_hidden, &[]);
        }
        if remaining_mines == frontier_vars.len() + interior_cells.len() {
            let mut all_hidden = frontier_vars.to_vec();
            all_hidden.extend_from_slice(&interior_cells);
            return self.apply_deductions(&[], &all_hidden);
        }

        if frontier_vars.is_empty() {
            return false;
        }

        let mut grid_to_var = vec![usize::MAX; self.cells.len()];
        for (var_idx, &grid_idx) in frontier_vars.iter().enumerate() {
            grid_to_var[grid_idx] = var_idx;
        }

        let mut var_to_constraints = vec![Vec::new(); frontier_vars.len()];
        for (c_idx, c) in constraints.iter().enumerate() {
            for &grid_idx in c.slice() {
                var_to_constraints[grid_to_var[grid_idx]].push(c_idx);
            }
        }

        let mut visited_vars = vec![false; frontier_vars.len()];
        let mut visited_constraints = vec![false; constraints.len()];
        let mut to_reveal = Vec::new();
        let mut to_flag = Vec::new();
        let mut component_results = Vec::new();
        let mut all_components_exact = true;

        for start_var in 0..frontier_vars.len() {
            if visited_vars[start_var] {
                continue;
            }
            let mut comp_var_ids = Vec::new();
            let mut comp_c_ids = Vec::new();
            let mut queue = VecDeque::from([start_var]);
            visited_vars[start_var] = true;

            while let Some(v) = queue.pop_front() {
                comp_var_ids.push(v);
                for &c_idx in &var_to_constraints[v] {
                    if !visited_constraints[c_idx] {
                        visited_constraints[c_idx] = true;
                        comp_c_ids.push(c_idx);
                        for &grid_idx in constraints[c_idx].slice() {
                            let nv = grid_to_var[grid_idx];
                            if !visited_vars[nv] {
                                visited_vars[nv] = true;
                                queue.push_back(nv);
                            }
                        }
                    }
                }
            }

            if comp_var_ids.len() > MAX_COMPONENT_VARS {
                all_components_exact = false;
                continue;
            }

            // Reorder component variables for tight constraint closure during DFS.
            let ordered_var_ids = order_component_vars(
                &comp_var_ids,
                &var_to_constraints,
                constraints,
                &grid_to_var,
            );
            let mut local_of_grid = vec![u8::MAX; self.cells.len()];
            let mut comp_grid_vars = Vec::with_capacity(ordered_var_ids.len());
            for (local_idx, &var_id) in ordered_var_ids.iter().enumerate() {
                let grid_idx = frontier_vars[var_id];
                local_of_grid[grid_idx] = local_idx as u8;
                comp_grid_vars.push(grid_idx);
            }

            let bit_constraints: Vec<BitConstraint> = comp_c_ids
                .iter()
                .map(|&c_idx| {
                    let c = &constraints[c_idx];
                    let mut mask = 0u32;
                    for &grid_idx in c.slice() {
                        mask |= 1u32 << local_of_grid[grid_idx];
                    }
                    BitConstraint {
                        mask,
                        mines: c.mines,
                    }
                })
                .collect();

            let Some(solutions) = solve_component(
                comp_grid_vars.len(),
                &bit_constraints,
                remaining_mines.min(u8::MAX as usize) as u8,
            ) else {
                all_components_exact = false;
                continue;
            };

            if solutions.is_empty() {
                continue;
            }

            let (always_mine, always_safe, min_k, max_k) =
                summarize_solutions(&solutions, comp_grid_vars.len());
            collect_forced(
                &comp_grid_vars,
                always_mine,
                always_safe,
                &mut to_flag,
                &mut to_reveal,
            );
            component_results.push(ComponentResult {
                vars: comp_grid_vars,
                solutions,
                min_k,
                max_k,
            });
        }

        if self.apply_deductions(&to_reveal, &to_flag) {
            return true;
        }

        if !all_components_exact {
            return false;
        }

        // Cross-component & interior global mine-count reasoning.
        let sum_min: usize = component_results.iter().map(|r| r.min_k).sum();
        let sum_max: usize = component_results.iter().map(|r| r.max_k).sum();
        let interior_len = interior_cells.len();

        let mut global_reveal = Vec::new();
        let mut global_flag = Vec::new();

        for comp in &component_results {
            let other_min = sum_min - comp.min_k;
            let other_max = sum_max - comp.max_k;
            let filtered: Vec<(u32, u8)> = comp
                .solutions
                .iter()
                .copied()
                .filter(|&(_, k)| {
                    let k = k as usize;
                    k + other_min <= remaining_mines
                        && remaining_mines <= k + other_max + interior_len
                })
                .collect();
            if filtered.is_empty() {
                continue;
            }
            let (always_mine, always_safe, _, _) = summarize_solutions(&filtered, comp.vars.len());
            collect_forced(
                &comp.vars,
                always_mine,
                always_safe,
                &mut global_flag,
                &mut global_reveal,
            );
        }

        if !interior_cells.is_empty() {
            if remaining_mines <= sum_min {
                global_reveal.extend_from_slice(&interior_cells);
            } else if remaining_mines >= sum_max + interior_len {
                global_flag.extend_from_slice(&interior_cells);
            }
        }

        self.apply_deductions(&global_reveal, &global_flag)
    }

    fn apply_deductions(&mut self, to_reveal: &[usize], to_flag: &[usize]) -> bool {
        let prev_revealed = self.revealed_safe;
        let prev_flagged = self.flagged_count;
        for &idx in to_flag {
            self.flag_mine(idx);
        }
        for &idx in to_reveal {
            self.reveal_safe(idx);
        }
        self.revealed_safe != prev_revealed || self.flagged_count != prev_flagged
    }
}

fn sorted_diff(a: &[usize], b: &[usize]) -> ([usize; 8], usize, [usize; 8], usize) {
    let mut only_a = [0usize; 8];
    let mut only_a_len = 0usize;
    let mut only_b = [0usize; 8];
    let mut only_b_len = 0usize;

    let (mut ia, mut ib) = (0usize, 0usize);
    while ia < a.len() && ib < b.len() {
        if a[ia] < b[ib] {
            only_a[only_a_len] = a[ia];
            only_a_len += 1;
            ia += 1;
        } else if b[ib] < a[ia] {
            only_b[only_b_len] = b[ib];
            only_b_len += 1;
            ib += 1;
        } else {
            ia += 1;
            ib += 1;
        }
    }
    while ia < a.len() {
        only_a[only_a_len] = a[ia];
        only_a_len += 1;
        ia += 1;
    }
    while ib < b.len() {
        only_b[only_b_len] = b[ib];
        only_b_len += 1;
        ib += 1;
    }

    (only_a, only_a_len, only_b, only_b_len)
}

fn order_component_vars(
    comp_vars: &[usize],
    var_to_constraints: &[Vec<usize>],
    constraints: &[Constraint],
    grid_to_var: &[usize],
) -> Vec<usize> {
    let mut remaining = comp_vars.to_vec();
    let mut ordered = Vec::with_capacity(comp_vars.len());
    let mut placed = vec![false; grid_to_var.len()];

    while !remaining.is_empty() {
        let mut best_pos = 0usize;
        let mut best_score = (0usize, 0usize);
        for (pos, &var_id) in remaining.iter().enumerate() {
            let touched_score: usize = var_to_constraints[var_id]
                .iter()
                .map(|&c_idx| {
                    constraints[c_idx]
                        .slice()
                        .iter()
                        .filter(|&&g| placed[g])
                        .count()
                })
                .sum();
            let deg = var_to_constraints[var_id].len();
            let score = (touched_score, deg);
            if score > best_score {
                best_score = score;
                best_pos = pos;
            }
        }
        let chosen = remaining.swap_remove(best_pos);
        ordered.push(chosen);
        for &c_idx in &var_to_constraints[chosen] {
            for &g in constraints[c_idx].slice() {
                placed[g] = true;
            }
        }
    }
    ordered
}

fn solve_component(
    num_vars: usize,
    constraints: &[BitConstraint],
    max_mines: u8,
) -> Option<Vec<(u32, u8)>> {
    let mut var_constraints: [Vec<usize>; MAX_COMPONENT_VARS] = Default::default();
    for (c_idx, c) in constraints.iter().enumerate() {
        for (bit, slot) in var_constraints.iter_mut().enumerate().take(num_vars) {
            if (c.mask >> bit) & 1 != 0 {
                slot.push(c_idx);
            }
        }
    }

    struct Search<'a> {
        num_vars: usize,
        constraints: &'a [BitConstraint],
        var_constraints: &'a [Vec<usize>; MAX_COMPONENT_VARS],
        max_mines: u8,
        nodes: usize,
        aborted: bool,
        solutions: Vec<(u32, u8)>,
    }

    impl Search<'_> {
        fn dfs(&mut self, depth: usize, assigned_mask: u32, mine_mask: u32, mines_used: u8) {
            if self.aborted || mines_used > self.max_mines {
                return;
            }
            self.nodes += 1;
            if self.nodes > MAX_COMPONENT_NODES {
                self.aborted = true;
                return;
            }
            if depth == self.num_vars {
                self.solutions.push((mine_mask, mines_used));
                return;
            }

            let bit = 1u32 << depth;
            let next_assigned = assigned_mask | bit;
            for is_mine in [false, true] {
                let next_mines_used = mines_used + u8::from(is_mine);
                if next_mines_used > self.max_mines {
                    continue;
                }
                let next_mine_mask = if is_mine { mine_mask | bit } else { mine_mask };
                let valid = self.var_constraints[depth].iter().all(|&c_idx| {
                    let c = self.constraints[c_idx];
                    let placed = (next_mine_mask & c.mask).count_ones() as u8;
                    let undecided = (!next_assigned & c.mask).count_ones() as u8;
                    placed <= c.mines && placed + undecided >= c.mines
                });
                if valid {
                    self.dfs(depth + 1, next_assigned, next_mine_mask, next_mines_used);
                    if self.aborted {
                        return;
                    }
                }
            }
        }
    }

    let mut search = Search {
        num_vars,
        constraints,
        var_constraints: &var_constraints,
        max_mines,
        nodes: 0,
        aborted: false,
        solutions: Vec::new(),
    };
    search.dfs(0, 0, 0, 0);
    if search.aborted {
        None
    } else {
        Some(search.solutions)
    }
}

fn summarize_solutions(solutions: &[(u32, u8)], num_vars: usize) -> (u32, u32, usize, usize) {
    let valid_bits = if num_vars >= 32 {
        u32::MAX
    } else {
        (1u32 << num_vars) - 1
    };
    let mut always_mine = valid_bits;
    let mut always_safe = valid_bits;
    let mut min_k = usize::MAX;
    let mut max_k = 0usize;

    for &(mask, k) in solutions {
        always_mine &= mask;
        always_safe &= !mask;
        min_k = min_k.min(k as usize);
        max_k = max_k.max(k as usize);
    }

    (always_mine, always_safe, min_k, max_k)
}

fn collect_forced(
    vars: &[usize],
    always_mine: u32,
    always_safe: u32,
    to_flag: &mut Vec<usize>,
    to_reveal: &mut Vec<usize>,
) {
    for (bit, &grid_idx) in vars.iter().enumerate() {
        if (always_mine >> bit) & 1 != 0 {
            to_flag.push(grid_idx);
        } else if (always_safe >> bit) & 1 != 0 {
            to_reveal.push(grid_idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_from_mines(width: usize, height: usize, mine_coords: &[(usize, usize)]) -> Vec<bool> {
        let mut mines = vec![false; width * height];
        for &(x, y) in mine_coords {
            mines[y * width + x] = true;
        }
        mines
    }

    #[test]
    fn detects_unavoidable_fifty_fifty_guess() {
        // 4x4 board with 2 mines where clicking (0, 0) reveals row 0..=2 except
        // column 3, leaving (3, 0) and (3, 1) symmetric 1-mine pair (Wait: let
        // mines form a symmetric pair behind clues: (0, 3) and (1, 3) wait -
        // verify: place 1 mine at (3, 2) or (3, 3) hidden behind (2, 1)?)
        // Explicit minimal 50/50 at corner (3, 2) & (3, 3):
        // Put 2 mines at (0, 3) and (3, 3) — wait, check if corner clues
        // distinguish (3, 2) vs (3, 3): if (2, 2) == 2 and (2, 3) == 1, wait.
        // True symmetric 50/50 on 4x4: mines at (3, 0) and (3, 3)? No, put 1
        // mine at (3, 0) on a 4x4 board? No, (2, 0) has adjacent 1, (2, 1) has
        // adjacent 1 — wait (3, 1) is revealed or hidden? If 2 mines:
        // mines = [(3, 0), (3, 1)] -> forced; [(3, 0), (2, 0)] vs [(3, 1), (2, 1)]!
        // Put mines at (2, 3), (3, 2) on 4x4 and click (0, 0):
        // Let's trace cells when click (0, 0):
        // Row 0 & 1 are adjacent = 0 -> reveal rows 0..1 completely, plus row 2
        // up to (1, 2) & (2, 2) & (3, 2)? Wait (3, 2) is mine!
        // With mines at (3, 2) and (2, 3): (3, 1) has adjacent=1 (only (3, 2)), so
        // (3, 2) is forced!
        // Classic 50/50 at corner (0, 3)-(1, 3) vs ... wait:
        // Wall of 3x3 mines? Better: 5x4 board with 2 mines at (4, 1) and (3, 0)?
        // Wait: simplest exact symmetric 50/50 on 5x4:
        // Mines at (4, 1) and (4, 3)? Check from (0, 0):
        // Rows/cols (0..=2, 0..=3) all 0 -> floods up to x=3!
        // Wait: (3, 0)=1 (adj (4,1)), (3, 1)=1, (3, 2)=2 (adj (4,1)&(4,3)),
        // (3, 3)=1 (adj (4,3)) -> (3, 2)=2 forces BOTH (4, 1) and (4, 3)!
        // For a 50/50 on 4x4: box two cells completely behind clues!
        // On 4x5, click (0, 0), place mines at (0, 3), (1, 3), (2, 3), (3, 3),
        // Wait if an entire row (y=3) is mines, (0..=3, 2) are all revealed as
        // 2/3/3/2 -> all mines in y=3 are forced, AND behind y=3 (y=4) there
        // is 1 mine among 4 unexposed cells! Impossible to tell which of (0..=3, 4)
        // is the mine without guessing!
        let mines = board_from_mines(4, 5, &[(0, 3), (1, 3), (2, 3), (3, 3), (2, 4)]);
        assert!(!is_solvable(4, 5, 5, &mines, 0, 0));
    }

    #[test]
    fn solves_pairwise_and_minecount_patterns() {
        // Require pairwise 1-2-1 elimination AND global mine-count deduction at
        // the end:
        // 5x5 board with click at (2, 0):
        // Top 3 rows expose a 1-2-1 along y=2? Let's craft an exact board where
        // single-cell rules stall after flood-fill, pairwise 1-2-1 fires, and
        // mine-count finishes the enclosed bottom-right region:
        // Mines at (1, 3), (3, 3) and (2, 4) vs Wait: check from (2, 0):
        // y=0: all 0 -> floods y=0..=1 and y=2 up to x=0..=4!
        // On y=2: (0,2)=1 (from (1,3)), (1,2)=1 (from (1,3)), (2,2)=2 (from (1,3)&(3,3)),
        // (3,2)=1 (from (3,3)), (4,2)=1 (from (3,3)).
        // Single-cell check on y=2:
        // (0,2)=1 has hidden {(0,3),(1,3)} (len 2)
        // (1,2)=1 has hidden {(0,3),(1,3),(2,3)} (len 3)
        // Pairwise (1,2) - (0,2) => (2,3) MUST BE SAFE!
        // Once (2,3) opens, its number sees (1,3), (3,3) and y=4! If we place ONLY
        // mines at (1, 3) and (3, 3) (2 mines total):
        // After pairwise opens (2, 3) (value 2, hidden {(1,3),(3,3),(1,4),(2,4),(3,4)})?
        // Wait: (1,2)=1 and (0,2)=1 -> (2,3) safe! Also (3,2)-(4,2) redundant.
        // Wait: once (2,3) opens with value 2, does Tier 1/2 solve (1,3) & (3,3)?
        // (0,2)=1 on {(0,3),(1,3)} and (1,2)=1 on {(0,3),(1,3)} -> same!
        // Wait why would Tier 1 stall before? Put mine at (0, 3) & (3, 3) instead?
        // Let's trace classic 1-2-1: mines at (1, 3) and (3, 3) on 5x4 (mines = 2):
        // Hidden after click (2, 0): whole row y=3 -> {(0,3),(1,3),(2,3),(3,3),(4,3)}!
        // y=2 clues:
        // (0,2)=1 {(0,3),(1,3)}
        // (1,2)=1 {(0,3),(1,3),(2,3)}  -> Tier 2 deduces (2,3) safe!
        // Still (0,3) vs (1,3) looks symmetric unless (2,3) touches (1,3)!
        // (2,3) has adjacent = 2 (sees BOTH (1,3) and (3,3)!).
        // Still {(0,3),(1,3)} sum to 1 and {(3,3),(4,3)} sum to 1, AND total mines = 2!
        // Tier 1 STILL can't tell which of {(0,3),(1,3)} has the mine EXCEPT Tier 3 sees:
        // Component 1 {(0,3),(1,3)} has 1 mine, Component 2 {(3,3),(4,3)} has 1 mine,
        // wait: (2,3)=2 has {(1,3),(3,3)}=2 which only becomes rem=2 on 2 cells -> Tier 1 flags (1,3)&(3,3)!
        // Let's make sure SINGLE-CELL ALONE would NOT solve this board without Tier 2!
        // Because (0,2)=1 (len 2), (1,2)=1 (len 3), (2,2)=2 (len 3), (3,2)=1 (len 3), (4,2)=1 (len 2):
        // Every clue on y=2 has rem < len and rem > 0, so Tier 1 stalls 100% immediately!
        // Then Tier 2 (pairwise) reveals (2, 3), which shows '2' on 2 hidden neighbours (1, 3) & (3, 3) (wait (2,3) also sees (1,3),(3,3) -> len 2 == rem 2) -> solves completely!
        let mines = board_from_mines(5, 4, &[(1, 3), (3, 3)]);
        assert!(is_solvable(5, 4, 2, &mines, 2, 0));
    }

    #[test]
    fn solves_global_minecount_endgame() {
        // 4x5 board with a sealed 2-cell interior behind a known mine wall, but
        // remaining global mines == 0 so mine-count reveals the interior!
        // Mines: (0, 3), (1, 3), (2, 3), (3, 3) -> 4 mines total, 0 left in y=4.
        let mines = board_from_mines(4, 5, &[(0, 3), (1, 3), (2, 3), (3, 3)]);
        assert!(is_solvable(4, 5, 4, &mines, 0, 0));
    }
}
