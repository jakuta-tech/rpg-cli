extern crate dirs;

use crate::character::Character;
use crate::location::Location;
use crate::log;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{fs, io, path};

#[derive(Debug)]
pub enum Error {
    GameOver,
    NoDataFile,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Game {
    pub player: Character,
    pub location: Location,
}

// TODO factor out all dir/file management code
fn rpg_dir() -> path::PathBuf {
    dirs::home_dir().unwrap().join(".rpg")
}

fn data_file() -> path::PathBuf {
    rpg_dir().join("data")
}

impl Game {
    pub fn new() -> Self {
        Self {
            location: Location::home(),
            player: Character::player(),
        }
    }

    pub fn load() -> Result<Self, Error> {
        let data = fs::read(data_file()).or(Err(Error::NoDataFile))?;
        let game: Game = bincode::deserialize(&data).unwrap();
        Ok(game)
    }

    pub fn save(&self) -> Result<(), io::Error> {
        let rpg_dir = rpg_dir();
        if !rpg_dir.exists() {
            fs::create_dir(&rpg_dir).unwrap();
        }

        let data = bincode::serialize(&self).unwrap();
        fs::write(data_file(), &data)
    }

    pub fn reset(&self) {
        let rpg_dir = rpg_dir();
        if rpg_dir.exists() {
            fs::remove_dir_all(&rpg_dir).unwrap();
        }
    }

    // TODO document
    // TODO use a less awkward name for such a central function.
    // maybe just `move`
    pub fn walk_towards(&mut self, dest: &Location) -> Result<(), Error> {
        while self.location != *dest {
            self.location.walk_towards(&dest);
            if self.location.is_home() {
                let recovered = self.player.heal();
                log::heal(&self.player, &self.location, recovered);
            } else if let Some(mut enemy) = self.maybe_spawn_enemy() {
                return self.battle(&mut enemy);
            }
        }
        Ok(())
    }

    fn maybe_spawn_enemy(&self) -> Option<Character> {
        if self.should_enemy_appear() {
            let level = self.enemy_level();
            let enemy = Character::new("enemy", level);
            log::enemy_appears(&enemy, &self.location);
            Some(enemy)
        } else {
            None
        }
    }

    fn should_enemy_appear(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen_ratio(1, 3)
    }

    fn enemy_level(&self) -> i32 {
        let distance: i32 = self.location.distance_from_home();
        let mut rng = rand::thread_rng();
        let random_delta = rng.gen_range(-1..2);
        std::cmp::max(self.player.level / 2 + distance - 1 + random_delta, 1)
    }

    fn battle(&mut self, enemy: &mut Character) -> Result<(), Error> {
        // this could be generalized to player vs enemy parties
        let (mut pl_accum, mut en_accum) = (0, 0);
        let player = &mut self.player;
        let mut xp = 0;

        while !enemy.is_dead() {
            pl_accum += player.speed;
            en_accum += enemy.speed;

            if pl_accum >= en_accum {
                let (new_xp, damage) = Self::attack(player, enemy);
                xp += new_xp;

                log::player_attack(&enemy, &self.location, damage);
                pl_accum = -1;
            } else {
                let (_, damage) = Self::attack(enemy, player);
                log::enemy_attack(&player, &self.location, damage);
                en_accum = -1;
            }

            if player.is_dead() {
                log::battle_lost(&player, &self.location);
                return Err(Error::GameOver);
            }
        }

        // TODO gather gold for real
        let gold = 100;
        let level_up = player.add_experience(xp);
        log::battle_won(&player, &self.location, xp, level_up, gold);

        Ok(())
    }

    /// Inflict damage from attacker to receiver, return the inflicted
    /// damage and the experience that will be gain if the battle is won
    fn attack(attacker: &mut Character, receiver: &mut Character) -> (i32, i32) {
        let damage = attacker.damage(&receiver);
        receiver.receive_damage(damage);
        let xp = attacker.xp_gained(&receiver, damage);
        (damage, xp)
    }
}
