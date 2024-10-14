#![allow(unused)]

use core::num;
use std::{fmt::Result, vec};

use rand::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
// const EXTRA_BASE_PROB: f64 = 0.4;

const SCORE_FROM_SECOND:     f64 = 0.60; // MLB approximate average scoring rate from second on a single = 0.6
const SCORE_FROM_FIRST:      f64 = 0.42; // MLB approximate average scoring rate from first on a double = 0.42
const FIRST_TO_THIRD:        f64 = 0.33; // MLB approximate average advancement rate from first on a single = 0.33
const EXTRA_BASE_FROM_THROW: f64 = 0.07; // This is when there's a runner on 2nd and the throw goes home, single gets the extra base
const THROWN_OUT_AT_HOME:    f64 = 0.05; // Probability the runner will be thrown out at home on a single from second

fn main() {
   
    let start_time = std::time::Instant::now();
    
    let players = create_players(0.260, 0.5001, 0.260, 0.6501);
    // dbg!(&players);
    // dbg!(&players.len());

    let num_innings = 1_000_000;

    
    let player_results: Vec<PlayerResult> = players.par_iter()
        .map(|player| simulate_batter(player, num_innings))
        .collect();
    
    let file_name = format!("{}\\{}", std::env::current_dir().unwrap().display(), "players.csv");
    let file = std::fs::OpenOptions::new().create(true).write(true).open(file_name).expect("Coultn't open the file for writing");
    
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(file);

    for player in player_results {
        csv_writer.serialize(player).expect("Couldn't serialize the batter!");
    }   

    
    let elapsed_time = start_time.elapsed();
    println!("Running simulation took {} seconds.", elapsed_time.as_secs());

}



fn create_players (obp_low: f64, obp_high: f64, slg_low: f64, slg_high: f64) -> Vec<Player> {
    
    let mut players: Vec<Player> = vec![];

    let mut avg: f64 = 0.0;
    let mut obp: f64 = obp_low;
    let mut slg: f64 = slg_low;

    let avg_low: f64 = 0.190;

    let mut obp_loop: Vec<f64> = vec![];
    let mut slg_loop: Vec<f64> = vec![];

    while obp <= obp_high {
        obp_loop.push(obp);
        obp += 0.005;
    };
    
    while slg <= slg_high {
        slg_loop.push(slg);
        slg += 0.005;
    };

    for obp in obp_loop.clone().into_iter() {
        for slg in slg_loop.clone().into_iter() {
            // dbg!(obp, slg);
            avg = avg_low;
            while avg <= obp {
                // dbg!(avg);
                players.push(Player {avg, obp, slg});
                avg += 0.005;
            }
        }
    }
    
    players
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InningResult {
    runs: usize,
    plate_appearances: usize,
    at_bats: usize,
    hits: usize,
    total_bases: usize,
    walks: usize,
}

fn simulate_batter (player: &Player, num_innings: usize) -> PlayerResult {
    
    let (mut runs, mut at_bats, mut plate_appearances, mut walks, mut hits, mut total_bases) = (0,0,0,0,0,0);
    for _ in 0 .. num_innings {
        let inning_result = simulate_inning(&player);
        runs += inning_result.runs;
        at_bats += inning_result.at_bats;
        plate_appearances += inning_result.plate_appearances;
        walks += inning_result.walks;
        hits += inning_result.hits;
        total_bases += inning_result.total_bases;
    }

    //     let runs_per_9 = runs as f64 / num_innings as f64 * 9.0;
    //     let avg_sim = hits as f64 / at_bats as f64;
    //     let slg_sim = total_bases as f64 / at_bats as f64;
    //     let obp_sim = (hits + walks) as f64 / plate_appearances as f64;
    //     let avg = player.avg;
    //     println!("Player with {avg:.03} batting average scored {runs_per_9:.02} runs per 9 innings");
    //     println!("Player hit {avg_sim:.03}\\{obp_sim:0.03}\\{slg_sim:0.03} in the simulation");  

    PlayerResult {
        avg: player.avg,
        obp: player.obp,
        slg: player.slg,
        num_innings,
        runs,
    }
}

/// Simulate an inning, assuming every player is exactly the same.
/// This will return the number of runs for the inning
fn simulate_inning (player: &Player) -> InningResult {

    let mut base_out_state = BaseOutState::new();
    let mut inning_result = InningResult {
        runs: 0,
        plate_appearances: 0,
        at_bats: 0,
        hits: 0,
        total_bases: 0,
        walks: 0
    };

    while base_out_state.outs < 3 {
        let result: PlateAppearanceResult = player.distribution().into();
        match result {
            PlateAppearanceResult::Out => {
                inning_result.plate_appearances += 1;
                inning_result.at_bats +=1;
            },
            PlateAppearanceResult::Walk => {
                inning_result.walks += 1;
                inning_result.plate_appearances +=1;
            }
            PlateAppearanceResult::Single => {
                inning_result.plate_appearances += 1;
                inning_result.at_bats +=1;
                inning_result.hits +=1;
                inning_result.total_bases +=1;
            },
            PlateAppearanceResult::Double => {
                inning_result.plate_appearances += 1;
                inning_result.at_bats +=1;
                inning_result.hits +=1;
                inning_result.total_bases +=2;
            },
            PlateAppearanceResult::Triple => {
                inning_result.plate_appearances += 1;
                inning_result.at_bats +=1;
                inning_result.hits +=1;
                inning_result.total_bases +=3;
            },
            PlateAppearanceResult::HomeRun => {
                inning_result.plate_appearances += 1;
                inning_result.at_bats +=1;
                inning_result.hits +=1;
                inning_result.total_bases +=4;
            },
        }

        let mut new_runs = 0;
        (base_out_state, new_runs) = base_out_state.move_runners(result);
        inning_result.runs += new_runs;
    }

    inning_result

}

fn test_result_base_out_state (player: Player) {
    // let result: PlateAppearanceResult = player.distribution().into();  
    let mut base_out_state = BaseOutState::new();
    let mut runs: usize = 0;

    let results = [
    PlateAppearanceResult:: Single,
    PlateAppearanceResult:: Walk,
    PlateAppearanceResult:: Double,
    PlateAppearanceResult:: Walk,
    PlateAppearanceResult:: HomeRun,
    ];

    for result in results {
        // let result: PlateAppearanceResult = player.distribution().into();
        let mut new_runs = 0;
        (base_out_state, new_runs) = base_out_state.move_runners(result);
        println! ("{result:?} {new_runs}");  
        runs += new_runs;
        
        
    }
    dbg!(&base_out_state, runs);


}

fn test_calibration (num_runs: usize) {
    let player = Player {
        avg: 0.315,
        obp: 0.365,
        slg: 0.510,
    };


    let mut singles = 0;
    let mut doubles = 0;
    let mut triples = 0;
    let mut home_runs = 0;
    let mut walks = 0;
    let mut outs = 0;

    for _ in 0 .. num_runs {

        let result: PlateAppearanceResult = player.distribution().into();  
        match result {
            PlateAppearanceResult::Out =>       {outs += 1},
            PlateAppearanceResult::Single =>    {singles += 1},
            PlateAppearanceResult::Double =>    {doubles += 1},
            PlateAppearanceResult::Triple =>    {triples += 1},
            PlateAppearanceResult::HomeRun =>   {home_runs += 1},
            PlateAppearanceResult::Walk =>      {walks += 1},
        }
    }
    
    println!("Outs: {outs}");
    println!("Singles: {singles}");
    println!("Doubles: {doubles}");
    println!("Triples: {triples}");
    println!("Home Runs: {home_runs}");
    println!("Walks: {walks}");
}

#[derive(Serialize, Debug)]
struct PlayerResult {
    avg: f64,
    obp: f64,
    slg: f64,
    num_innings: usize,
    runs: usize,
}

#[derive(Debug)]
struct BaseOutState {
    on_1st: bool,
    on_2nd: bool,
    on_3rd: bool,
    outs: u8,
}

impl BaseOutState {
    fn new() -> Self {
        Self {
            on_1st: false,
            on_2nd: false,
            on_3rd: false,
            outs: 0,
        }
    }
    fn move_runners (self, result: PlateAppearanceResult) -> (Self, usize) {

        let (mut on_1st, mut on_2nd, mut on_3rd) = (self.on_1st, self.on_2nd, self.on_3rd);
        let mut runs = 0;

        let outs = if result == PlateAppearanceResult::Out {self.outs + 1} else {self.outs};

        let mut rng = thread_rng();
        
        let first_to_third = rng.gen::<f64>() < FIRST_TO_THIRD;
        let second_to_home = rng.gen::<f64>() < SCORE_FROM_SECOND;
        let first_to_home  = rng.gen::<f64>() < SCORE_FROM_FIRST;
        let single_extra   = rng.gen::<f64>() < EXTRA_BASE_FROM_THROW;
        
        
        if result == PlateAppearanceResult::Walk {
            on_1st = true;
            on_2nd = self.on_1st || ( self.on_2nd && !self.on_1st) ;
            on_3rd = (self.on_1st && self.on_2nd) || ( self.on_3rd && !self.on_2nd) || ( self.on_3rd && !self.on_1st);
            if self.on_1st && self.on_2nd && self.on_3rd {runs = 1}; 
        }

        if result == PlateAppearanceResult::Single {

            
            if single_extra && on_2nd {
                on_1st = false;
                on_2nd = true;
                on_3rd = self.on_1st || (self.on_2nd && !second_to_home);
                if self.on_2nd && second_to_home {runs += 1}; 
                if self.on_3rd               {runs += 1}; 
            }
            else {
                on_1st = true;
                on_2nd = self.on_1st && !first_to_third;
                on_3rd = (self.on_1st && first_to_third) || (self.on_2nd && !second_to_home);
                if self.on_2nd && second_to_home {runs += 1}; 
                if self.on_3rd               {runs += 1}; 
            }
        }

        if result == PlateAppearanceResult::Double {

            on_1st = false;
            on_2nd = true;
            on_3rd = self.on_1st && !first_to_home;
            if self.on_1st && first_to_home {runs += 1}; 
            if self.on_2nd               {runs += 1}; 
            if self.on_3rd               {runs += 1}; 
        }
        
        if result == PlateAppearanceResult::Triple {

            on_1st = false;
            on_2nd = false;
            on_3rd = true;
            if self.on_1st               {runs += 1}; 
            if self.on_2nd               {runs += 1}; 
            if self.on_3rd               {runs += 1}; 
        }

        if result == PlateAppearanceResult::HomeRun {

            on_1st = false;
            on_2nd = false;
            on_3rd = false;
            runs = 1;
            if self.on_1st               {runs += 1}; 
            if self.on_2nd               {runs += 1}; 
            if self.on_3rd               {runs += 1}; 
        }

        (Self {on_1st, on_2nd, on_3rd, outs}, runs)

    }

}

#[derive(Debug, Clone)]
struct Player {
    avg: f64,
    obp: f64,
    slg: f64,
}

// For simplicity we'll ignore triples. 
// It's theoretically possible that this will impact the simulation,
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlateAppearanceResult {
    Walk,
    Single,
    Double,
    Triple,
    HomeRun,
    Out,
}

#[derive(Debug, Clone, Copy)]
struct PlateAppearanceDistribution {
    walk: f64,
    single: f64,
    double: f64,
    triple: f64,
    home_run: f64,
}

impl From<PlateAppearanceDistribution> for PlateAppearanceResult {
    fn from(dist: PlateAppearanceDistribution) -> Self {
        let mut rng = thread_rng();
        let x: f64 = rng.gen();

        if x < dist.walk {PlateAppearanceResult::Walk}
        else if x < dist.single { PlateAppearanceResult::Single}
        else if x < dist.double { PlateAppearanceResult::Double}
        else if x < dist.triple { PlateAppearanceResult::Triple}
        else if x < dist.home_run { PlateAppearanceResult::HomeRun}
        else {PlateAppearanceResult::Out}
    }
}

impl Player {
    fn distribution (&self) -> PlateAppearanceDistribution {

        // Using a little bit of arithmetic, we can determine that the BB/AB ratio = (OBP-BA)(1-OBP)
        // We can then compute the per-plate appearance probabilities
        // We assume a 56-5-39 ratio of 2B-3B-HR based on what happened at the league level in 2024

        let bb_per_ab = (self.obp - self.avg) / (1.0 - self.obp);
        let walk = bb_per_ab / (1.0 + bb_per_ab);
        let double =   (self.slg - self.avg) * 0.56 / 1.83 / (1.0 + bb_per_ab);
        let triple =   (self.slg - self.avg) * 0.05 / 1.83 / (1.0 + bb_per_ab);
        let home_run = (self.slg - self.avg) * 0.39 / 1.83 / (1.0 + bb_per_ab);
        let single = self.avg / (1.0 + bb_per_ab) - double - home_run - triple;

        assert!((walk + double + triple + home_run + single - self.obp).abs() < 0.001);

        // We stack the probabilities
        PlateAppearanceDistribution {
            walk,
            single:   walk + single,
            double:   walk + single + double,
            triple:   walk + single + double + triple,
            home_run: walk + single + double + triple + home_run,
        }
        

    }
}
