use plotters::prelude::*;
use std::cmp;

const EJECTION_BALANCE: u64 = 16_000_000_000;
const FAR_FUTURE_EPOCH: u64 = 18446744073709551615;
const CHURN_LIMIT_QUOTIENT: u64 = 65536;
const MIN_PER_EPOCH_CHURN_LIMIT: u64 = 4;
const MAX_SEED_LOOKAHEAD: u64 = 4;
const INACTIVITY_SCORE_BIAS: u64 = 4;
const INACTIVITY_SCORE_RECOVERY_RATE: u64 = 16;
const INACTIVITY_PENALTY_QUOTIENT: u64 = 16777216;

struct State {
    epoch: u64,
    validators: Vec<Validator>,
    balances: Vec<u64>,
    inactivity_scores: Vec<u64>,
    participating_count: u64,
    participating: Vec<bool>,
    exit_queue_epoch: u64,
    exit_queue_churn: u64,
    active_count_prev_epoch: u64,
    active_balance: u64,
    active_participating_balance: u64,
    max_active_inactivity_score: u64,
}

impl State {
    fn new() -> Self {
        State {
            epoch: 0,
            validators: vec![],
            balances: vec![],
            inactivity_scores: vec![],
            participating_count: 0,
            participating: vec![],
            exit_queue_epoch: 0,
            exit_queue_churn: 0,
            active_count_prev_epoch: 0,
            active_balance: 0,
            active_participating_balance: 0,
            max_active_inactivity_score: 0,
        }
    }

    fn add_validator(&mut self, participating: bool, initial_balance: u64) {
        self.validators.push(Validator::new());
        self.balances.push(initial_balance);
        self.inactivity_scores.push(0);
        self.participating.push(participating);
        self.active_balance += initial_balance;
        self.active_count_prev_epoch += 1;
        if participating {
            self.participating_count += 1;
            self.active_participating_balance += initial_balance;
        }
    }

    fn is_participating(&self, index: usize) -> bool {
        self.participating[index]
    }

    fn is_in_inactivity_leak(&self) -> bool {
        3 * self.active_participating_balance < 2 * self.active_balance
    }

    fn get_validator_churn_limit(&self) -> u64 {
        cmp::max(
            MIN_PER_EPOCH_CHURN_LIMIT,
            self.active_count_prev_epoch / CHURN_LIMIT_QUOTIENT,
        )
    }

    fn initiate_validator_exit(&mut self, index: usize) {
        let validator = self.validators.get(index).unwrap();
        if validator.exit_epoch != FAR_FUTURE_EPOCH {
            return;
        }

        let min_exit_epoch = compute_activation_exit_epoch(self.epoch);
        if self.exit_queue_epoch < min_exit_epoch {
            self.exit_queue_epoch = min_exit_epoch;
            self.exit_queue_churn = 0;
        }

        if self.exit_queue_churn >= self.get_validator_churn_limit() {
            self.exit_queue_epoch += 1;
            self.exit_queue_churn = 0;
        }

        self.exit_queue_churn += 1;
        self.validators.get_mut(index).unwrap().exit_epoch = self.exit_queue_epoch as u64;
    }
}

struct Validator {
    exit_epoch: u64,
}

impl Validator {
    fn new() -> Self {
        Validator {
            exit_epoch: FAR_FUTURE_EPOCH,
        }
    }

    fn is_active_validator(&self, epoch: u64) -> bool {
        epoch < self.exit_epoch
    }
}

fn compute_activation_exit_epoch(epoch: u64) -> u64 {
    epoch + 1 + MAX_SEED_LOOKAHEAD
}

fn process_registry_updates_single_pass(state: &mut State, index: usize) {
    // Process activation eligibility and ejections
    if state.validators[index].is_active_validator(state.epoch)
        && state.balances[index] <= EJECTION_BALANCE
    {
        state.initiate_validator_exit(index); // initiate_validator_exit would need to be adjusted to work in this context
    }
}

fn process_inactivity_updates_single_pass(state: &mut State, index: usize) {
    // Increase the inactivity score of inactive validators
    if state.is_participating(index) {
        if state.inactivity_scores[index] > 0 {
            state.inactivity_scores[index] -= std::cmp::min(1, state.inactivity_scores[index]);
        }
    } else {
        state.inactivity_scores[index] += INACTIVITY_SCORE_BIAS;
    }
    // Decrease the inactivity score of all eligible validators during a leak-free epoch
    if !state.is_in_inactivity_leak() {
        state.inactivity_scores[index] -= std::cmp::min(
            INACTIVITY_SCORE_RECOVERY_RATE,
            state.inactivity_scores[index],
        );
    }
}

fn process_rewards_and_penalties_single_pass(state: &mut State, index: usize) {
    if !state.is_participating(index) {
        let penalty_numerator = state.balances[index] * state.inactivity_scores[index];
        let penalty_denominator = INACTIVITY_SCORE_BIAS * INACTIVITY_PENALTY_QUOTIENT;
        state.balances[index] -= penalty_numerator / penalty_denominator;
    }
}

fn process_epoch_single_pass(state: &mut State) {
    let mut active_count_prev_epoch = 0;
    let mut active_balance = 0;
    let mut active_participating_balance = 0;
    let mut max_active_inactivity_score = 0;

    let previous_epoch = state.epoch.saturating_sub(1);

    for index in 0..state.validators.len() {
        let is_active_prev_epoch = state.validators[index].is_active_validator(previous_epoch);
        if is_active_prev_epoch {
            process_inactivity_updates_single_pass(state, index);
            process_rewards_and_penalties_single_pass(state, index);
        }
        process_registry_updates_single_pass(state, index);

        if is_active_prev_epoch {
            active_count_prev_epoch += 1;
            active_balance += state.balances[index];
            if state.is_participating(index) {
                active_participating_balance += state.balances[index];
            }
            // Track for stopping condition
            max_active_inactivity_score =
                max_active_inactivity_score.max(state.inactivity_scores[index])
        }
    }

    state.epoch += 1;
    state.active_count_prev_epoch = active_count_prev_epoch;
    state.active_balance = active_balance;
    state.active_participating_balance = active_participating_balance;
    state.max_active_inactivity_score = max_active_inactivity_score;
}

fn compute_min_max_avg(numbers: &[u64]) -> (u64, u64, f64) {
    if numbers.is_empty() {
        panic!("Vector is empty");
    }

    let mut min = numbers[0];
    let mut max = numbers[0];
    let mut sum = 0_u64;

    for &number in numbers {
        if number < min {
            min = number;
        }
        if number > max {
            max = number;
        }
        sum += number;
    }

    let avg = sum as f64 / numbers.len() as f64;

    (min, max, avg)
}

fn run_test(offline_percent: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Example usage
    let mut state = State::new();
    state.add_validator(true, 32_000_000_000);
    // Add more logic for handling epochs, activations, exits, and penalties.

    let n = 1_000_000;
    let initial_balance = 32_000_000_000;

    let start_index_non_participant = ((100 - offline_percent) * n) / 100;

    for i in 0..n {
        state.add_validator(i < start_index_non_participant - 1, initial_balance);
    }

    let mut min_balances = vec![];
    let mut avg_balances = vec![];
    let mut max_balances = vec![];
    let mut min_inactivity_scores = vec![];
    let mut avg_inactivity_scores = vec![];
    let mut max_inactivity_scores = vec![];
    let mut inactivity_leak_stop_epoch = None;
    let mut activate_validators = vec![];
    let mut participation = vec![];
    let mut active_balance = vec![];
    let mut active_participating_balance = vec![];

    loop {
        process_epoch_single_pass(&mut state);

        // Record metrics
        let (min, max, avg) = compute_min_max_avg(&state.balances[start_index_non_participant..]);
        min_balances.push(min);
        avg_balances.push(avg);
        max_balances.push(max);
        let (min, max, avg) =
            compute_min_max_avg(&state.inactivity_scores[start_index_non_participant..]);
        min_inactivity_scores.push(min);
        avg_inactivity_scores.push(avg);
        max_inactivity_scores.push(max);

        if !state.is_in_inactivity_leak() && inactivity_leak_stop_epoch.is_none() {
            inactivity_leak_stop_epoch = Some(state.epoch);
        }

        activate_validators.push(state.active_count_prev_epoch);
        participation.push(state.active_participating_balance as f64 / state.active_balance as f64);
        active_balance.push(state.active_balance);
        active_participating_balance.push(state.active_participating_balance);

        // Stop when finality is recovered and there are no more penalties applied to active
        // validators = max inactivity score has returned to zero.
        if !state.is_in_inactivity_leak() && state.max_active_inactivity_score == 0 {
            break;
        }
    }

    println!(
        "
offline_percent:        {offline_percent}
inactivity_leak_stop:   {inactivity_leak_stop_epoch:?}
balance[end]:           {} {} {}
inactivity_scores[end]: {} {} {}
activate_validators[e]: {}
participation[end]:     {}
state_end_epoch:        {}
exit_queue_epoch:       {}
    ",
        min_balances.last().unwrap(),
        avg_balances.last().unwrap(),
        max_balances.last().unwrap(),
        min_inactivity_scores.last().unwrap(),
        avg_inactivity_scores.last().unwrap(),
        max_inactivity_scores.last().unwrap(),
        activate_validators.last().unwrap(),
        participation.last().unwrap(),
        state.epoch,
        state.exit_queue_epoch,
    );

    draw_line(
        &format!("active_balance_{offline_percent}.png"),
        &active_balance
            .iter()
            .map(|x| *x as f32)
            .collect::<Vec<f32>>(),
    )?;
    draw_line(
        &format!("avg_balances_{offline_percent}.png"),
        &avg_balances.iter().map(|x| *x as f32).collect::<Vec<f32>>(),
    )?;
    draw_line(
        &format!("avg_inactivity_scores_{offline_percent}.png"),
        &avg_inactivity_scores
            .iter()
            .map(|x| *x as f32)
            .collect::<Vec<f32>>(),
    )?;
    draw_line(
        &format!("participation_{offline_percent}.png"),
        &participation
            .iter()
            .map(|x| *x as f32)
            .collect::<Vec<f32>>(),
    )?;

    Ok(())
}

fn draw_line(filename: &str, data: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let max = data.iter().max_by(|a, b| a.partial_cmp(b).unwrap());
    let min = data.iter().min_by(|a, b| a.partial_cmp(b).unwrap());

    let mut chart = ChartBuilder::on(&root)
        .caption(filename, ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(100)
        .build_cartesian_2d(
            0..data.len() as u32,
            *min.unwrap() as f32..*max.unwrap() as f32,
        )?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            data.iter().enumerate().map(|(x, y)| (x as u32, *y as f32)),
            &RED,
        ))?
        .label(filename)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for offline_percent in [35, 40, 50, 60, 70, 80, 90] {
        run_test(offline_percent)?;
    }

    Ok(())
}
