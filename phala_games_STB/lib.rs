#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use pink_extension as pink;

#[pink::contract(env=PinkEnvironment)]
mod phala_games_STB {
    // use core::fmt::Error;
    use super::pink;
    use ink::prelude::{
        format,
        string::{String, ToString},
        vec::Vec,
    };
    use ink::storage::traits::StorageLayout;
    use ink::storage::Mapping;
    use pink::{http_get, PinkEnvironment};
    use scale::{Decode, Encode};

    // use pink_utils::attestation;

    use core::cmp;
    // use crate::alloc::string::ToString; //used at   account[1..last_elem_num].to_string()
    use scale::CompactAs;
    use sp_arithmetic::FixedU128;

    use serde::Deserialize;
    // you have to use crates with `no_std` support in contract.
    use serde_json_core;

    const CLAIM_PREFIX: &str = "This gist is owned by address: 0x";
    const ADDRESS_LEN: usize = 64;

    #[derive(Default, Debug, Clone, scale::Encode, scale::Decode, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct TicketsInfo {
        ticket_id: u32,
        owner: Option<AccountId>,
        tickets_coordinates: (u32, u32),
        distance_from_target: u128,
    }

    #[derive(Default, Debug, Clone, scale::Encode, scale::Decode, PartialEq)]
    #[cfg_attr(feature = "std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct HallOfFame {
        ticket_id: u32,
        owner: Option<AccountId>,
        tickets_coordinates: (u32, u32),
        distance_from_target: u128,
        prize_money: Balance,
        timestamp: u64,
        competition_number: u32,
        start_time: u64,
        end_time: u64,
        number_of_tickets: u32,
        number_of_players: u32,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        HttpRequestFailed,
        InvalidResponseBody,
    }

    /// Type alias for the contract's result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct PhalaGamesSTB {
        admin: AccountId,
        // linked_users: Mapping<String, AccountId>,
        game_state: bool,
        image_hash: String,
        start_time: u64,
        end_time: u64,
        ticket_cost: Balance,
        next_ticket_id: u32,
        x_sum: u32,
        y_sum: u32,
        tickets_mapping: Mapping<u32, TicketsInfo>, //for ticket id+1123 the owner, coordinates are (x1,y1)
        players_mapping: Mapping<AccountId, Vec<u32>>, //accountId oX12 owns tickets 1,2,3
        players: Vec<AccountId>,
        ordered_ticket_ids: Vec<u32>,
        winners_ids: Vec<u32>,
        balances: Mapping<AccountId, Balance>,
        total_pot: Balance,
        total_net_pot: Balance,
        total_fees: Balance,
        fees_percent: Balance,
        competition_number: u32,
        wisdom_of_crowd_coordinates: (u32, u32),
        hall_of_fame_vec: Vec<HallOfFame>,
    }

    impl PhalaGamesSTB {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                // linked_users: Mapping::default(),
                game_state: false,
                image_hash: Default::default(),
                start_time: Default::default(),
                end_time: Default::default(),
                ticket_cost: 1_000_000_000_000,
                next_ticket_id: 0,
                x_sum: 0,
                y_sum: 0,
                tickets_mapping: Mapping::default(),
                players_mapping: Mapping::default(),
                players: Default::default(),
                ordered_ticket_ids: Default::default(),
                winners_ids: Default::default(),
                balances: Mapping::default(),
                total_pot: 0,
                total_net_pot: 0,
                total_fees: 0,
                fees_percent: 20,
                competition_number: 0,
                wisdom_of_crowd_coordinates: Default::default(),
                hall_of_fame_vec: Default::default(),
            }
        }

        /// Configure the Game  
        #[ink(message)]
        pub fn config_game(
            &mut self,
            image_hash: String,
            start_time: u64,
            end_time: u64,
            ticket_cost: Balance,
            fees_percent: Balance,
        ) {
            assert!(
                image_hash != String::from(""),
                "image_hash must not be empty"
            );
            assert!(
                start_time < end_time || (start_time == end_time && end_time == 0),
                "start_time must be < end_time"
            );

            // assert!(ticket_cost > 0, "ticket_cost must be > 0");

            self.image_hash = image_hash;
            if start_time > 0 {
                self.start_time = start_time;
            } else {
                self.start_time = self.env().block_timestamp();
            }

            if end_time > 0 {
                self.end_time = end_time;
            } else {
                self.end_time = self.start_time + (10 * 60 * 1000); //10mins
            }
            if ticket_cost > 0 {
                self.ticket_cost = ticket_cost;
            }
            if fees_percent > 0 {
                self.fees_percent = fees_percent;
            }
        }

        /// Update Game state to start, end , find winners and make payments
        #[ink(message)]
        pub fn check_game(&mut self) {
            //TO UNCOMMENT IN THE END
            // if !self.game_state {
            if !self.game_state
                && self.env().block_timestamp() > self.start_time
                && self.env().block_timestamp() < self.end_time
            {
                self.game_state = true;
                self.competition_number += 1;
                self.image_hash = Default::default();
                self.tickets_mapping = Mapping::default();
                self.players_mapping = Mapping::default();
                self.players = Default::default();
                self.ordered_ticket_ids = Default::default();
                self.winners_ids = Default::default();
            } else if self.game_state && self.env().block_timestamp() > self.end_time {
                self.game_state = false;

                //CALCUALTE DISTANCES TO DETERMINE THE WINNER
                self.calculate_distances();

                //FIND WINNERS
                self.find_winers(1);
                let winning_ticket: TicketsInfo = self.get_tickets_mapping(self.winners_ids[0]);

                //UPDATE HALL OF FAME
                let winner_hof = HallOfFame {
                    ticket_id: winning_ticket.ticket_id,
                    owner: winning_ticket.owner,
                    tickets_coordinates: winning_ticket.tickets_coordinates,
                    distance_from_target: winning_ticket.distance_from_target,

                    prize_money: self.total_net_pot,
                    timestamp: self.env().block_timestamp(),
                    competition_number: self.competition_number,
                    start_time: self.start_time,
                    end_time: self.end_time,
                    number_of_tickets: self.ordered_ticket_ids.len() as u32,
                    number_of_players: self.players.len() as u32,
                };
                self.hall_of_fame_vec.push(winner_hof);

                //MAKE PAYMENTS
                self.make_payments(winning_ticket.owner.unwrap());
            }
        }

        /// Get Game state, image_hash, start and end time and ticket cost
        #[ink(message)]
        pub fn get_game_stats(&self) -> (bool, String, u64, u64, Balance, u32) {
            (
                self.game_state,
                self.image_hash.clone(),
                self.start_time,
                self.end_time,
                self.ticket_cost,
                self.competition_number,
            )
        }

        #[ink(message)]
        pub fn get_block_ts(&self) -> u64 {
            self.env().block_timestamp()
        }

        /// Get all player AccountIds  
        #[ink(message)]
        pub fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        /// Get Sums For Testing Only  
        #[ink(message)]
        pub fn get_sums(&self) -> (u32, u32) {
            //ToDo convert to function after testing or only for Admin
            (self.x_sum, self.y_sum)
        }

        /// For a given AccountId get all ticket Ids  
        #[ink(message)]
        pub fn get_players_mapping(&self, account: AccountId) -> Vec<u32> {
            self.players_mapping.get(&account).unwrap_or_default()
        }

        /// Give me ticket Id and get all ticket details  
        #[ink(message)]
        pub fn get_tickets_mapping(&self, ticket_id: u32) -> TicketsInfo {
            self.tickets_mapping.get(&ticket_id).unwrap_or_default()
        }

        /// Get all ticket coordinates  
        #[ink(message)]
        pub fn get_all_tickets(&self) -> Vec<(u32, u32)> {
            // assert!(self.players.len() > 0, "must have at least 1 player");
            let mut all_tickets: Vec<(u32, u32)> = Vec::new();

            for player_address in &self.players {
                let player_ticket_ids: Vec<u32> = self.get_players_mapping(*player_address);

                for ticketid in &player_ticket_ids {
                    let ticket_coordinates: (u32, u32) = self
                        .tickets_mapping
                        .get(&ticketid)
                        .unwrap()
                        .tickets_coordinates;

                    all_tickets.push(ticket_coordinates)
                }
            }
            all_tickets
        }

        #[ink(message)]
        pub fn get_ordered_ticket_ids(&self) -> Vec<u32> {
            self.ordered_ticket_ids.clone()
        }

        /// Get Solution  
        #[ink(message)]
        pub fn get_wisdom_of_crowd_coordinates(&self) -> (u32, u32) {
            self.wisdom_of_crowd_coordinates
        }

        /// Calcualte Solution TO BE CALLED AFTER GAME ENDS
        #[ink(message)]
        pub fn calculate_wisdom_of_crowd_coordinates(&mut self) -> (u32, u32) {
            // fn calculate_wisdom_of_crowd_coordinates(&self) -> (u32, u32) {
            // assert!(
            //     self.env().block_timestamp() > self.end_time,
            //     "only to run after game expires"
            // );

            //ToDo To be called only by Admin and only after game ends
            let woc_x = (self.x_sum / self.next_ticket_id) as u32;
            let woc_y = (self.y_sum / self.next_ticket_id) as u32;
            self.wisdom_of_crowd_coordinates = (woc_x, woc_y);
            (woc_x, woc_y)
        }

        /// Find winners ticket ids TO TEST AGAIN
        #[ink(message)]
        pub fn find_winers(&mut self, number_of_winners: u32) {
            // fn find_winers(&mut self, number_of_winners: u32) {

            let possible_winners_ids = self.ordered_ticket_ids.clone();
            assert!(
                possible_winners_ids.len() > 0,
                "must have at least 1 ticket"
            );

            let mut numwinrs = (number_of_winners - 1) as usize;
            if numwinrs >= possible_winners_ids.len() {
                numwinrs = possible_winners_ids.len() - 1 as usize;
            }

            let mut winners_ids: Vec<u32> = Vec::new();

            for n in 0..=numwinrs {
                let winning_ticket: TicketsInfo = self.get_tickets_mapping(possible_winners_ids[n]);
                winners_ids.push(winning_ticket.ticket_id);
            }

            self.winners_ids = winners_ids;
        }

        /// Get winnign tickets TO TEST AGAIN
        #[ink(message)]
        pub fn get_winning_tickets(&self) -> Vec<TicketsInfo> {
            let winners_ids = self.winners_ids.clone();
            assert!(winners_ids.len() > 0, "must have at least 1 winner");

            let mut winning_tickets: Vec<TicketsInfo> = Vec::new();

            for n in 0..winners_ids.len() {
                let winning_ticket: TicketsInfo = self.get_tickets_mapping(winners_ids[n]);
                winning_tickets.push(winning_ticket);
            }
            winning_tickets
        }

        /// Get winners Vec of AccountIds TO TEST AGAIN
        #[ink(message)]
        pub fn get_winners_addresses(&self) -> Vec<AccountId> {
            let winners_ids = self.winners_ids.clone();
            assert!(winners_ids.len() > 0, "must have at least 1 winner");

            let mut winners_addresses: Vec<AccountId> = Vec::new();

            for n in 0..winners_ids.len() {
                let winning_ticket: TicketsInfo = self.get_tickets_mapping(winners_ids[n]);

                winners_addresses.push(winning_ticket.owner.unwrap());
            }
            winners_addresses
        }

        /// Submit new ticket  
        #[ink(message, payable)]
        pub fn submit_tickets(&mut self, tickets: Vec<(u32, u32)>) -> Result<()> {
            assert!(tickets.len() > 0, "must have at least 1 ticket");
            let caller: AccountId = self.env().caller();
            let endowment = self.env().transferred_value();
            //ticket_cost = 1_000_000_000_000
            let expected_value = (tickets.len() as u128) * self.ticket_cost;
            ink::env::debug_println!(
                "endowment {:?} expected_value {:?} ",
                endowment,
                expected_value
            );
            assert!(endowment == expected_value, "ticket are not paid");

            let balance = self.balances.get(caller).unwrap_or(0);
            self.balances.insert(caller, &(balance + endowment));

            self.total_pot += self.env().transferred_value();
            self.total_fees = (self.total_pot / 100) * self.fees_percent;
            self.total_net_pot = self.total_pot - self.total_fees;

            ink::env::debug_println!(
                "total_pot: {} total_net_pot: {} total_fees: {} ",
                self.total_pot,
                self.total_net_pot,
                self.total_fees
            );

            //Add Player
            if !self.players.contains(&caller) {
                ink::env::debug_println!(
                    "player {:?} is a new player and will be added to players Vec ",
                    caller,
                );
                self.players.push(caller);
            } else {
                ink::env::debug_println!(
                    "player {:?} is an existing player and will NOT be added to players Vec ",
                    caller,
                );
            }

            //Get players ticket_ids
            let mut player_ticketids = self.get_players_mapping(caller);

            for ticket in tickets {
                self.next_ticket_id += 1;
                let ticket_info = TicketsInfo {
                    ticket_id: self.next_ticket_id,
                    owner: Some(caller),
                    tickets_coordinates: ticket,
                    distance_from_target: 0,
                };

                //Add ticket tickets_mapping
                self.tickets_mapping
                    .insert(&self.next_ticket_id, &ticket_info);
                //Collect fresh ticket ids
                player_ticketids.push(self.next_ticket_id);
                //Update sums
                self.x_sum += ticket.0;
                self.y_sum += ticket.1;
            }
            //Add new ticket_ids to existing ones
            self.players_mapping.insert(&caller, &player_ticketids);

            Ok(())
        }

        /// Calculate distances of tickets from solution  
        #[ink(message)]
        pub fn calculate_distances(&mut self) {
            // assert!(self.players.len() > 0, "must have at least 1 player");
            let (woc_x, woc_y) = self.calculate_wisdom_of_crowd_coordinates();

            let mut all_tickets: Vec<TicketsInfo> = Vec::new();

            for player_address in &self.players {
                let player_ticket_ids: Vec<u32> = self.get_players_mapping(*player_address);

                for ticketid in &player_ticket_ids {
                    let mut tickt: TicketsInfo = self.tickets_mapping.get(&ticketid).unwrap();
                    let (t_x, t_y): (u32, u32) = tickt.tickets_coordinates;
                    let vert_dist: u32 = u32::pow((t_x - woc_x), 2);
                    let horiz_dist: u32 = u32::pow((t_y - woc_y), 2);
                    let sum_of_squares: u32 = (vert_dist + horiz_dist); // as u32;
                    let d1 = FixedU128::from_u32(sum_of_squares);
                    let d2 = FixedU128::sqrt(d1);
                    let distance = *d2.encode_as();

                    // let distancesq = vert_dist + horiz_dist; //f64::sqrt(vert_dist.pow(2) + horiz_dist.pow(2));
                    tickt.distance_from_target = distance;

                    self.tickets_mapping.insert(&ticketid, &tickt);

                    all_tickets.push(tickt);
                }
            }

            all_tickets.sort_by_key(|d| d.distance_from_target);
            let mut winners_ids: Vec<u32> = Vec::new();
            for n in 0..=(all_tickets.len() - 1) {
                winners_ids.push(all_tickets[n].ticket_id);
            }
            // self.winners = winners_ids;
            self.ordered_ticket_ids = winners_ids;
        }

        /// Admin of sc  
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin.clone()
        }

        /// Get square root  
        // #[ink(message)]
        // pub fn get_squareroot(&self, num: u32) -> u128 {
        fn get_squareroot(num: u32) -> u128 {
            let d1 = FixedU128::from_u32(num);
            let d2 = FixedU128::sqrt(d1);
            let d3 = *d2.encode_as();
            d3
        }

        /// Retrieve the balance of the caller.
        #[ink(message)]
        pub fn get_balance(&self, account: Option<AccountId>) -> Balance {
            let mut caller = self.env().caller();
            if account != None {
                caller = account.unwrap();
            }
            self.balances.get(caller).unwrap()
        }

        /// get_existential_deposit  
        #[ink(message)]
        pub fn get_existential_deposit(&self) -> Balance {
            self.env().minimum_balance() //1
        }

        /// check if accoutn is a contract  
        #[ink(message)]
        pub fn account_is_contract(&self, account: AccountId) -> bool {
            self.env().is_contract(&account)
        }

        /// get contract balance  
        #[ink(message)]
        pub fn get_contract_balance(&self) -> Balance {
            self.env().balance()
        }

        /// get_total_pot inclusive of fees
        #[ink(message)]
        pub fn get_total_pot(&self) -> Balance {
            self.total_pot
        }

        /// total_net_pot
        #[ink(message)]
        pub fn get_total_net_pot(&self) -> Balance {
            self.total_net_pot
        }

        /// total_fees
        #[ink(message)]
        pub fn get_total_fees(&self) -> Balance {
            self.total_fees
        }

        /// fees_percent by default 20 for 20%
        #[ink(message)]
        pub fn get_fees_percent(&self) -> Balance {
            self.fees_percent
        }

        fn make_payments(&mut self, account: AccountId) {
            ink::env::debug_println!(
                "pay_winners> caller {:?} this_contract: {:?} winnerAddress: {:?}",
                self.env().caller(),
                self.env().account_id(),
                account
            );
            let fess_to_transfer =
                self.env().balance() - self.total_net_pot - self.get_existential_deposit();
            self.env().transfer(account, self.total_net_pot).unwrap();
            self.env().transfer(self.admin, fess_to_transfer).unwrap();
            self.reset_game();
        }

        fn reset_game(&mut self) {
            self.game_state = false;
            // self.image_hash = Default::default();
            self.start_time = Default::default();
            self.end_time = Default::default();
            self.next_ticket_id = 0;
            self.x_sum = 0;
            self.y_sum = 0;
            // self.tickets_mapping = Mapping::default();
            // self.players_mapping = Mapping::default();
            // self.players = Default::default();
            // self.ordered_ticket_ids = Default::default();
            // self.winners_ids = Default::default();
            self.balances = Mapping::default();
            self.total_pot = 0;
            self.total_net_pot = 0;
            self.total_fees = 0;
        }

        // Get Hall of Fame of past winners
        #[ink(message)]
        pub fn get_hall_of_fame(&self) -> Vec<HallOfFame> {
            self.hall_of_fame_vec.clone()
        }
    }
}
