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
        ticket_id: u8,
        owner: Option<AccountId>,
        tickets_coordinates: (u8, u8),
        distance_from_target: u128,
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InvalidEthAddress,
        InvalidPrefixEthAddress,
        InvalidLengthEthAddress,
        HttpRequestFailed,
        InvalidResponseBody,
        BadOrigin,
        BadgeContractNotSetUp,
        InvalidUrl,
        RequestFailed,
        NoClaimFound,
        InvalidAddressLength,
        InvalidAddress,
        NoPermission,
        InvalidSignature,
        UsernameAlreadyInUse,
        AccountAlreadyInUse,
        FailedToIssueBadge,
    }

    /// Type alias for the contract's result type.
    pub type Result<T> = core::result::Result<T, Error>;

    // #[derive(SpreadAllocate)]
    // #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    #[ink(storage)]
    pub struct PhalaHttpAttestationGist {
        admin: AccountId,
        // attestation_verifier: attestation::Verifier,
        // attestation_generator: attestation::Generator,
        // linked_users: Mapping<String, ()>,
        linked_users: Mapping<String, AccountId>,
        game_state: bool,
        image_hash: String,
        start_time: u64,
        end_time: u64,
        ticket_cost: Balance,
        next_ticket_id: u8,
        x_sum: u8,
        y_sum: u8,
        tickets_mapping: Mapping<u8, TicketsInfo>, //for ticket id+1123 the owner, coordinates are (x1,y1)
        players_mapping: Mapping<AccountId, Vec<u8>>, //accountId oX12 owns tickets 1,2,3
        players: Vec<AccountId>,
        winners: Vec<u8>,
        winners_addresses: Vec<AccountId>,
    }

    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    pub struct EtherscanResponse<'a> {
        status: &'a str,
        message: &'a str,
        result: &'a str,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GistUrl {
        username: String,
        gist_id: String,
        filename: String,
    }

    // #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    // pub struct RandomResponse<'a> {
    //     result: &'a str,
    // }

    #[derive(Deserialize, Encode, Clone, Debug, PartialEq)]
    pub struct RandomResponse<'a> {
        status: &'a str,
        message: &'a str,
        result: &'a str,
    }

    #[derive(Clone, Encode, Decode, Debug)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct GistQuote {
        username: String,
        account_id: AccountId,
    }

    impl PhalaHttpAttestationGist {
        #[ink(constructor)]
        pub fn new() -> Self {
            // Create the attestation helpers
            // let (generator, verifier) = attestation::create(b"gist-attestation-key");
            // Save sender as the contract admin
            // let admin = Self::env().caller();

            Self {
                admin: Self::env().caller(),
                // attestation_generator: generator,
                // attestation_verifier: verifier,
                linked_users: Mapping::default(),
                game_state: false,
                image_hash: Default::default(),
                start_time: Default::default(),
                end_time: Default::default(),
                ticket_cost: 0,
                next_ticket_id: 0,
                x_sum: 0,
                y_sum: 0,
                tickets_mapping: Mapping::default(),
                players_mapping: Mapping::default(),
                players: Default::default(),
                winners: Default::default(),
                winners_addresses: Default::default(),
            }

            // ink_lang::utils::initialize_contract(|contract: &mut Self| {
            //     contract.admin = admin;
            //     contract.attestation_generator = generator;
            //     contract.attestation_verifier = verifier;
            //     contract.linked_users = Default::default();
            //     contract.game_state = false;
            //     contract.image_hash = Default::default();
            //     contract.start_time = Default::default();
            //     contract.end_time = Default::default();
            //     contract.ticket_cost = 0;
            //     contract.next_ticket_id = 0;
            //     contract.x_sum = 0;
            //     contract.y_sum = 0;
            //     contract.tickets_mapping = Default::default();
            //     contract.players_mapping = Default::default();
            //     contract.players = Default::default();
            //     contract.winners = Default::default();
            //     contract.winners_addresses = Default::default();
            // })
        }

        /// Configure the Game
        #[ink(message)]
        pub fn config_game(
            &mut self,
            image_hash: String,
            start_time: u64,
            end_time: u64,
            ticket_cost: Balance,
        ) {
            // assert!(
            //     image_hash != String::from(""),
            //     "image_hash must not be empty"
            // );
            // assert!(start_time < end_time, "start_time must be < end_time");
            // assert!(
            //     start_time > self.env().block_timestamp()
            //         && end_time > self.env().block_timestamp(),
            //     "start_time and end_time must be  > self.env().block_timestamp()"
            // );
            // assert!(ticket_cost > 0, "ticket_cost must be > 0");

            self.image_hash = image_hash;
            self.start_time = start_time;
            self.end_time = end_time;
            self.ticket_cost = ticket_cost;
        }

        /// Update Game state to start, end , call winners and payments
        #[ink(message)]
        pub fn check_game(&mut self) {
            if !self.game_state && self.env().block_timestamp() > self.start_time {
                self.game_state = true;
            } else if self.game_state && self.env().block_timestamp() > self.end_time {
                self.game_state = false;

                //CALL WINNERS
                //MAKE PAYMENTS
            }
        }

        /// Get Game state, image_hash, start and end time and ticket cost
        #[ink(message)]
        pub fn get_game_stats(&self) -> (bool, String, u64, u64, Balance) {
            (
                self.game_state,
                self.image_hash.clone(),
                self.start_time,
                self.end_time,
                self.ticket_cost,
            )
        }

        #[ink(message)]
        pub fn get_block_ts(&self) -> u64 {
            self.env().block_timestamp()
        }

        /// Get Sums For Testing Only
        #[ink(message)]
        pub fn get_sums(&self) -> (u8, u8) {
            //ToDo convert to function after testing or only for Admin
            (self.x_sum, self.y_sum)
        }

        /// Calcualte Solution
        #[ink(message)]
        pub fn get_wisdom_of_crowd_coordinates(&self) -> (u8, u8) {
            //ToDo To be called only by Admin
            let woc_x = (self.x_sum / self.next_ticket_id) as u8;
            let woc_y = (self.y_sum / self.next_ticket_id) as u8;

            (woc_x, woc_y)
        }

        /// Get all player AccountIds
        #[ink(message)]
        pub fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        /// Get winners Vec of TicketInfo and Vec of AccountIds
        #[ink(message)]
        pub fn get_winers(&self, number_of_winners: u32) -> (Vec<TicketsInfo>, Vec<AccountId>) {
            let winners_ids = self.winners.clone();
            // assert!(winners_ids.len() > 0, "must have at least 1 winner");
            let mut numwinrs = number_of_winners as usize;
            if numwinrs >= winners_ids.len() {
                numwinrs = winners_ids.len() - 1 as usize;
            }

            // self.winners[0..=numwinrs].to_vec()

            let mut winners: Vec<TicketsInfo> = Vec::new();
            let mut winners_addresses: Vec<AccountId> = Vec::new();

            // for n in 0..=numwinrs {
            //     let winning_ticket: TicketsInfo = self.get_tickets_mapping(winners_ids[n]);

            //     winners_addresses.push(winning_ticket.owner);
            //     winners.push(winning_ticket);
            // }
            // // self.winners_addresses = winners_addresses;
            // winners
            (winners, winners_addresses)
        }

        /// Get winners AccountIds for Onion payouts
        #[ink(message)]
        pub fn get_winners_addresses(&self) -> Vec<AccountId> {
            self.winners_addresses.clone()
        }

        /// Get all ticket coordinates
        #[ink(message)]
        pub fn get_all_tickets(&self) -> Vec<(u8, u8)> {
            // assert!(self.players.len() > 0, "must have at least 1 player");
            let mut all_tickets: Vec<(u8, u8)> = Vec::new();

            for player_address in &self.players {
                let player_ticket_ids: Vec<u8> = self.get_players_mapping(*player_address);

                for ticketid in &player_ticket_ids {
                    let ticket_coordinates: (u8, u8) = self
                        .tickets_mapping
                        .get(&ticketid)
                        .unwrap()
                        .tickets_coordinates;

                    all_tickets.push(ticket_coordinates)
                }
            }
            all_tickets
        }

        /// For a given AccountId get all ticket Ids
        #[ink(message)]
        pub fn get_players_mapping(&self, account: AccountId) -> Vec<u8> {
            self.players_mapping.get(&account).unwrap_or_default()
        }

        /// Give me ticket Id and get all ticket details
        #[ink(message)]
        pub fn get_tickets_mapping(&self, ticket_id: u8) -> TicketsInfo {
            self.tickets_mapping.get(&ticket_id).unwrap_or_default()
        }

        /// Submit new ticket
        #[ink(message)]
        pub fn submit_tickets(&mut self, tickets: Vec<(u8, u8)>) -> Result<()> {
            // assert!(tickets.len() > 0, "must have at least 1 ticket");
            let caller: AccountId = self.env().caller();

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

            // //Get players ticket_ids
            // let mut player_ticketids = self.get_players_mapping(caller);

            // for ticket in tickets {
            //     self.next_ticket_id += 1;
            //     let ticket_info = TicketsInfo {
            //         ticket_id: self.next_ticket_id,
            //         owner: caller,
            //         tickets_coordinates: ticket,
            //         distance_from_target: 0,
            //     };

            //     //Add ticket tickets_mapping
            //     self.tickets_mapping
            //         .insert(&self.next_ticket_id, &ticket_info);
            //     //Collect fresh ticket ids
            //     player_ticketids.push(self.next_ticket_id);
            //     //Update sums
            //     self.x_sum += ticket.0;
            //     self.y_sum += ticket.1;
            // }
            // //Add new ticket_ids to existing ones
            // self.players_mapping.insert(&caller, &player_ticketids);

            Ok(())
        }

        /// Calculate distances of tickets from solution
        #[ink(message)]
        pub fn calculate_distances(&mut self) {
            // assert!(self.players.len() > 0, "must have at least 1 player");
            let (woc_x, woc_y) = self.get_wisdom_of_crowd_coordinates();

            let mut all_tickets: Vec<TicketsInfo> = Vec::new();

            for player_address in &self.players {
                let player_ticket_ids: Vec<u8> = self.get_players_mapping(*player_address);

                for ticketid in &player_ticket_ids {
                    let mut tickt: TicketsInfo = self.tickets_mapping.get(&ticketid).unwrap();
                    let (t_x, t_y): (u8, u8) = tickt.tickets_coordinates;
                    let vert_dist: u8 = u8::pow((t_x - woc_x), 2);
                    let horiz_dist: u8 = u8::pow((t_y - woc_y), 2);
                    let sum_of_squares: u32 = (vert_dist + horiz_dist) as u32;
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
            let mut winners_ids: Vec<u8> = Vec::new();
            for n in 0..=(all_tickets.len() - 1) {
                winners_ids.push(all_tickets[n].ticket_id);
            }
            self.winners = winners_ids;
        }

        // /// The attestation generator
        // #[ink(message)]
        // pub fn get_attestation_generator(&self) -> attestation::Generator {
        //     self.attestation_generator.clone()
        // }

        // /// The attestation verifier
        // #[ink(message)]
        // pub fn get_attestation_verifier(&self) -> attestation::Verifier {
        //     self.attestation_verifier.clone()
        // }

        /// Admin of sc
        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin.clone()
        }

        /// Get square root
        #[ink(message)]
        pub fn get_squareroot(&self, num: u32) -> u128 {
            let d1 = FixedU128::from_u32(num);
            let d2 = FixedU128::sqrt(d1);
            let d3 = *d2.encode_as();
            d3
        }

        /// Http call to get a random integer
        #[ink(message)]
        pub fn get_random_int(&self, min: u8, max: u8) -> Result<String> {
            let resp = http_get!(format!(
                "https://www.random.org/integers/?num=1&min={}&max={}&col=1&base=10&format=plain&rnd=new",
                min, max
            ));

            if resp.status_code != 200 {
                return Err(Error::HttpRequestFailed);
            }

            let result: RandomResponse = serde_json_core::from_slice(&resp.body)
                .or(Err(Error::InvalidResponseBody))?
                .0;
            Ok(String::from(result.result))
            // Ok((result.result).parse::<u8>().unwrap())
            // Ok(result.result)
        }

        /// Parses a Github Gist url.
        /// - Returns a parsed [GistUrl] struct if the input is a valid url;
        /// - Otherwise returns an [Error].
        fn parse_ranmdom_url(url: &str) -> Result<GistUrl> {
            let path = url
                .strip_prefix("https://gist.githubusercontent.com/")
                .ok_or(Error::InvalidUrl)?;
            let components: Vec<_> = path.split('/').collect();
            if components.len() < 5 {
                return Err(Error::InvalidUrl);
            }
            Ok(GistUrl {
                username: components[0].to_string(),
                gist_id: components[1].to_string(),
                filename: components[4].to_string(),
            })
        }
    }
}
