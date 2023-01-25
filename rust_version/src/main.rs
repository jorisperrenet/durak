// use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashSet;
use std::collections::HashMap;
use std::thread;
use std::io::{stdin,stdout,Write};

mod tools;


#[derive(Clone)]
struct GameVals {
    is_end_state: bool,
    loser: String,  // name of the loser
    card_collection: tools::CardCollection,
    random_generator: StdRng,
    deck: Vec<tools::Card>,
    attackers: Vec<u8>,
    defender: u8,
    current_attacker: u8,
    player_to_play: u8,
    draw_order: Vec<u8>,
    attacker_to_start_throwing: u8,
    last_played_attacker: u8,
    reflected_trumps: Vec<(i8, i8)>,
    current_action: tools::Action,
    pairs_finished: Vec<((i8, i8), (i8, i8))>,
    cards_to_defend: Vec<(i8, i8)>,
    print_info: bool,
    players: Vec<tools::Player>,
    computer_shuffle: bool,
    main_attacker: String,
}

impl Default for GameVals {
    fn default() -> GameVals {
        let empty_deck = [Default::default(); 36];
        let rng_seed = rand::thread_rng().gen_range(0..10000000);
        GameVals {
            is_end_state: false,
            loser: Default::default(),
            card_collection: Default::default(),
            // random_generator: StdRng::seed_from_u64(222),
            random_generator: StdRng::seed_from_u64(rng_seed),
            deck: empty_deck.to_vec(),
            attackers: Vec::with_capacity(6),  // Max num players
            defender: 0,
            current_attacker: 0,
            player_to_play: 0,
            draw_order: Vec::with_capacity(6),  // Max num players
            attacker_to_start_throwing: 0,
            last_played_attacker: 0,
            reflected_trumps: Vec::with_capacity(9),  // Number of trumps
            current_action: Default::default(),
            pairs_finished: Vec::with_capacity(18),  // Half the cards
            cards_to_defend: Vec::with_capacity(4),  // Max reflects
            print_info: true,
            players: Vec::with_capacity(6),  // Max num players
            computer_shuffle: true,
            main_attacker: "??".to_string(),
        }
    }
}

trait GameTree {
    fn initialize(&mut self);
    fn new_trick(&mut self);
    fn fill_hand(&mut self, player_idx: &u8);
    fn fallback_identities(&self, player_idx: &u8) -> Vec<(i8, i8)>;
    fn can_throw(&self, throw: &Vec<&(i8, i8)>, fallback: &HashSet<&(i8, i8)>, player_idx: &u8) -> bool;
    fn choose_action(&mut self) -> tools::Action;
    fn get_random_action(&self) -> tools::Action;
    fn execute_action(&mut self, action: tools::Action);
    fn make_cards_known(&mut self, player_idx: &u8);
    fn allowed_plays(&self) -> Vec<tools::Action>;
    fn let_human_choose(&self, lines_to_remove: u32) -> tools::Action;
    fn possible_card_plays(&self, player_idx: &u8) -> HashSet<(i8, i8)>;
    fn next(&mut self);
}

impl GameTree for GameVals {
    fn initialize(&mut self) {
        // Check if the amount of players is correct
        // 1 player is too few, and with 6 players
        // the cards are precisely divided
        assert!(2 <= self.players.len());
        assert!(self.players.len() <= 6);

        // Initialize the last/bottom card of the deck
        let card;
        if self.computer_shuffle {
            let unknown = &self.card_collection.unknown;
            let idx = self.random_generator.gen_range(0..unknown.len());
            card = self.card_collection.unknown.swap_remove(idx);
            // self.card_collection.public.push(card);
        } else {
            let unknown = &self.card_collection.unknown;
            println!("Input the bottom card");
            let tmp = input_card(unknown.to_vec());
            print!("\x1b[F\x1b[J");
            card = (tmp.0 as i8, tmp.1 as i8);
            self.card_collection.unknown.retain(|x| *x != card);
        }
        self.deck[0].suit = card.0;
        self.deck[0].value = card.1;
        self.deck[0].is_unknown = false;
        self.deck[0].is_public = true;
        self.card_collection.non_public.remove(&card);

        // Check if the bottom value is an ace
        if self.deck[0].value == 8 {
            // If so, redeal
            if self.print_info {
                println!("There was an ace on the bottom, redealing...");
            }
            self.card_collection = Default::default();
            self.deck = [tools::Card { ..Default::default() }; 36].to_vec();
            self.initialize();
            // Stop from going further
            return
        }

        // Display the bottom card
        if self.print_info {
            println!("The bottom card is {}", self.deck[0]);
        }

        // Set the trump suit
        let trump_suit: i8 = self.deck[0].suit;
        self.card_collection.trump_suit = trump_suit;
        for card in &mut self.deck {
            card.trump_suit = trump_suit;
        }

        // Initialize the hand of each player
        let num_players = self.players.len();
        for p in 0..num_players {
            self.card_collection.hands.insert(
                p as u8,
                Vec::with_capacity(36),
            );
            self.fill_hand(&(p as u8));
        }

        // Initialize a new trick
        self.new_trick();
    }

    fn new_trick(&mut self) {
        // Find the main attacker
        let mut idx = 0;
        while self.players[idx].name != self.main_attacker {
            idx += 1;
        }
        // Initialize a new trick with main_attacker as starting player
        self.attackers = Default::default();
        let num_players = self.players.len();

        // A person is still in the game whenever his hand is not empty or if
        // he can draw card.
        for person_idx in 0..num_players {
            let p = ((idx + person_idx) % num_players) as u8;
            if self.deck.len() > 0 || self.card_collection.hands[&p].len() > 0 {
                self.attackers.push(p);
            }
        }
        // Get attacker and defender from attackers
        if self.attackers.len() == 0 {
            // Game has ended
            // There are no attackers, the last card got defended
            // and no one has any cards left, the last defender lost.
            self.is_end_state = true;
            self.loser = self.players[self.defender as usize].name.clone();
        } else if self.attackers.len() == 1 {
            // Game has ended
            // There is only one person left in the game, the loser
            self.is_end_state = true;
            self.loser = self.players[self.attackers[0] as usize].name.clone();
        } else {
            // Game may continue
            self.defender = self.attackers.remove(1);
            // The main attacker may start off as the player to play
            self.current_attacker = 0;
            self.player_to_play = self.attackers[0];
            // People must draw cards according to the draw order
            self.draw_order = self.attackers.clone();
            self.draw_order.push(self.defender);
            // All the trumps that were used to reflect this turn (they lost their life)
            self.attacker_to_start_throwing = Default::default();
            self.last_played_attacker = Default::default();
            self.reflected_trumps = Default::default();
        }
        // The action to perform
        self.current_action = tools::Action {
            name: "attack".to_string(),
            ..Default::default()
        };
        // Succesfully defended cards as (attack, defend) pairs
        self.pairs_finished = Default::default();
        self.cards_to_defend = Default::default();
    }

    fn fill_hand(&mut self, player_idx: &u8) {
        let hand = self.card_collection.hands.get_mut(player_idx).unwrap();
        let to_draw: i8 = 6 - hand.len() as i8;
        for _ in 0..to_draw {
            if self.deck.len() == 0 {
                break
            }
            let card_drawn = self.deck.pop().unwrap();
            if (card_drawn.suit, card_drawn.value) != (-1, -1) {
                // Public card
                hand.push((card_drawn.suit, card_drawn.value, true));
            } else {
                // Unknown card
                hand.push((-1, -1, false));
            }
        }
    }

    fn make_cards_known(&mut self, player_idx: &u8) {
        // Ask for the card values
        let hand = self.card_collection.hands.get_mut(player_idx).unwrap();
        for card in hand {
            // Check if card unknown
            if card == &mut (-1, -1, false) {
                if self.computer_shuffle {
                    let idx = self.random_generator.gen_range(0..self.card_collection.unknown.len());
                    let info = self.card_collection.unknown.remove(idx);
                    *card = (info.0, info.1, false);  // Private card
                } else {
                    println!("");
                    println!("What card(s) did player {:?} draw?", self.players[*player_idx as usize]);
                    let info = input_card(self.card_collection.unknown.to_vec());
                    print!("\x1b[F\x1b[J");
                    print!("\x1b[F\x1b[J");
                    let mut suits = "♣♠♥♦".chars();
                    let mut values = "6789*JQKA".chars();
                    println!("You typed {}{}", suits.nth(info.0).unwrap(), values.nth(info.1).unwrap());
                    let tmp = (info.0 as i8, info.1 as i8);
                    // println!("{:?}", self.card_collection.unknown);
                    let hand = &mut self.card_collection.unknown;
                    if let Some(index) = hand.iter().position(|val| (val.0, val.1) == tmp) {
                        hand.swap_remove(index);
                    }
                    // println!("{:?}", self.card_collection.unknown);
                    *card = (info.0 as i8, info.1 as i8, false);  // Private card
                }
            }
        }
    }

    fn fallback_identities(&self, player_idx: &u8) -> Vec<(i8, i8)> {
        // Get the fallback cards, i.e. if a card is unknown
        // the possible identities of the card, that means
        // every non-public card minus the hand of the current player
        let non_public = &mut (self.card_collection.non_public.clone());
        for card in self.card_collection.hands.get(player_idx).unwrap() {
            if card.2 == false {
                // A private card of the player (the current player knows the identity of this card)
                non_public.remove(&(card.0, card.1));
            }
        }

        return non_public.iter().map(|x| *x).collect();
    }

    fn possible_card_plays(&self, player_idx: &u8) -> HashSet<(i8, i8)> {
        let mut poss_plays = HashSet::new();
        for card in &self.card_collection.hands[player_idx] {
            if card == &(-1, -1, false) {
                // This card can be any of the unknown cards (to the player)
                let tmp = self.fallback_identities(player_idx);
                poss_plays.extend(&tmp);
            } else {
                // This card is either public or private and known to this person
                poss_plays.insert((card.0, card.1));
            }
        }
        return poss_plays;
    }

    fn can_throw(&self, throw: &Vec<&(i8, i8)>, fallback: &HashSet<&(i8, i8)>, player_idx: &u8) -> bool {
        // Check if player_idx can throw away the cards in throw
        // Where each card has fallback identity fallback
        let mut poss = Vec::new();
        let mut num_fallbacks = 0;
        for card in self.card_collection.hands.get(player_idx).unwrap() {
            if card == &(-1, -1, false) {
                // Card is unkown
                num_fallbacks += 1;
            } else {
                if throw.contains(&&(card.0, card.1)) {
                    poss.push(HashSet::from([(card.0, card.1).clone()]))
                }
            }
        }
        // Easy case, poss is not big enough to consist of len(throw) cards
        if poss.len() + num_fallbacks < throw.len() {
            return false;
        }
        // Add the minimum amount of fallbacks needed
        for _ in 0..num_fallbacks.min(throw.len()) {
            poss.push(fallback.iter().map(|x| **x).collect());
        }
        // Greedy approach: take the nth card from the first allowed poss
        let mut found;
        for c in throw {
            found = false;
            for idx in 0..poss.len() {
                if poss[idx].contains(c) {
                    poss.swap_remove(idx);
                    found = true;
                    break
                }
            }
            if ! found {
                return false;
            }
        }
        return true;
    }

    fn allowed_plays(&self) -> Vec<tools::Action> {
        // We enumerate the possible actions of
        let player = self.player_to_play;
        // who will perform
        let action = &self.current_action;

        let mut poss_actions = Vec::new();
        if action.name == "attack".to_string() {
            // The player must attack the defender
            assert_eq!(self.attackers[self.current_attacker as usize], player);

            // List the possible plays of the player
            let mut poss_plays = self.possible_card_plays(&player);

            if self.pairs_finished.len() > 0 {
                // There is a pair on the table thus you can pass on attacking
                poss_actions.push(tools::Action {
                    name: "passattack".to_string(),
                    ..Default::default()
                });
                // If you do not pass, you must play cards with the same
                // value as those that lie on the table.
                let mut values_on_table: HashSet<i8> = self.pairs_finished.iter().map(|x| x.0.1).collect();
                values_on_table.extend(&self.pairs_finished.iter().map(|x| x.1.1).collect::<HashSet<i8>>());
                poss_plays.retain(|&pair| values_on_table.contains(&pair.1));
            }
            // Check if you are allowed to make another pile
            if self.card_collection.hands[&self.defender].len() > 0 {
                // Iterate through the cards you can play
                for (suit, value) in poss_plays {
                    poss_actions.push(tools::Action {
                        name: "attack".to_string(),
                        card: (suit, value),
                        ..Default::default()
                    });
                }
            }
        } else if action.name == "defend".to_string() {
            // We need to defend the first card from the single ones
            let to_defend = self.cards_to_defend[0];

            // We iterate through each card in the hand to see if we can use
            // it to defend the to_defend card.
            let mut unknown_count = 0;
            for card in &self.card_collection.hands[&self.player_to_play] {
                // Find the possible identities of this card
                let identity;
                if card == &(-1, -1, false) {
                    // This card is unknown
                    identity = self.fallback_identities(&self.player_to_play);
                    if unknown_count > 0 {
                        // These identities have already been checked
                        // and added to possible actions.
                        continue;
                    }
                    unknown_count += 1;
                } else {
                    identity = vec![(card.0, card.1)];
                }
                // Iterate through the identity to check which we can play
                for (suit, value) in identity {
                    // Reflecting
                    // Only if there are no finished pairs you can reflect
                    if self.pairs_finished.len() == 0 {
                        // Check if you are allowed to make another pile with reflecting
                        // The hypothetical new defender becoms
                        let new_defender = self.attackers[1 % (self.attackers.len())];
                        // The new defender must be able to defend all cards (if he wants)
                        // with the amount of cards in his hand.
                        let max_new_piles: i8 = self.card_collection.hands[&new_defender].len() as i8 - self.cards_to_defend.len() as i8;
                        // If we reflect by playing the card we create another pile
                        if max_new_piles >= 1 {
                            // Check if you can reflect the to_defend card with this card
                            if value == to_defend.1 {
                                poss_actions.push(tools::Action {
                                    name: "reflect".to_string(),
                                    card: (suit, value),
                                    ..Default::default()
                                })
                            }
                        }
                        // If we reflect by showing the trump card no extra pile is made
                        if max_new_piles >= 0 {
                            // Check if you can reflect the to_defend card with this card
                            if value == to_defend.1 && suit == self.card_collection.trump_suit {
                                if ! self.reflected_trumps.contains(&(suit, value)) {
                                    poss_actions.push(tools::Action {
                                        name: "reflecttrump".to_string(),
                                        card: (suit, value),
                                        ..Default::default()
                                    })
                                }
                            }
                        }
                    }

                    // Defending
                    // Check if you can defend the to_defend card with this card
                    if to_defend.0 != suit {
                        if suit == self.card_collection.trump_suit {
                            // You can always play a trump on a non-trump card to win
                            poss_actions.push(tools::Action {
                                name: "defend".to_string(),
                                card: (suit, value),
                                ..Default::default()
                            })
                        }
                    } else {
                        // If the cards are of the same suit, the higher one wins
                        if value > to_defend.1 {
                            poss_actions.push(tools::Action {
                                name: "defend".to_string(),
                                card: (suit, value),
                                ..Default::default()
                            })
                        }
                    }
                }
            }
            // You're always allowed to pick up the cards
            poss_actions.push(tools::Action {
                name: "take".to_string(),
                ..Default::default()
            });
        } else if action.name == "throwcards".to_string() {
            // List the possible throws of the player
            let mut poss_throws = self.possible_card_plays(&player);
            // You can only throw cards with the same value as those on the table
            let mut values_on_table: HashSet<i8> = self.pairs_finished.iter().map(|x| x.0.1).collect();
            values_on_table.extend(&self.pairs_finished.iter().map(|x| x.1.1).collect::<HashSet<i8>>());
            values_on_table.extend(&self.cards_to_defend.iter().map(|x| x.1).collect::<HashSet<i8>>());
            poss_throws.retain(|&pair| values_on_table.contains(&pair.1));

            // The amount of cards you can throw equals the number of cards in the hand
            // of the defender (originally) minus the amount of piles.
            // Hence, the number of current cards in the hand minus the amount of cards to defend.
            let available_throws = self.card_collection.hands[&self.defender].len() - self.cards_to_defend.len();
            // If 0 cards are thrown
            poss_actions.push(tools::Action {
                name: "throwcards".to_string(),
                ..Default::default()  // Throws no cards
            });
            // If more than 0 cards are thrown
            let max_throws = available_throws.min(poss_throws.len()).min(self.card_collection.hands[&self.player_to_play].len());
            let mut fallback: HashSet<&(i8, i8)> = HashSet::new();
            let tmp;
            let tmp2: HashSet<&(i8, i8)>;
            // let tmp2: HashSet<(i8, i8)>;
            // let to_add = &mut self.card_collection.hands[&self.player_to_play].clone();
            // let other;
            if max_throws > 0 {
                tmp = self.fallback_identities(&self.player_to_play);
                tmp2 = tmp.iter().collect::<HashSet<&(i8, i8)>>();
                fallback = tmp2;
                // Also include the public cards in the hand of the player
                // tmp2 = tmp.iter().map(|x| *x).collect();
                // to_add.retain(|x| x.2);
                // other = to_add.iter().map(|x| (x.0, x.1)).collect::<HashSet<(i8, i8)>>();
                // fallback = tmp2.union(&other).collect();
            }
            // Iterate through all subsets of the poss_throws
            // with minimum length 1 and maximum length max_throws (inclusive)
            let mut idx;
            let mut throw;
            let mut poss_throws_lst: Vec<&(i8, i8)> = poss_throws.iter().collect();
            poss_throws_lst.sort_by(|a, b| a.1.cmp(&b.1));
            poss_throws_lst.sort_by(|a, b| a.0.cmp(&b.0));
            for mut s in 1..1<<poss_throws.len() {
                if (s as u32).count_ones() as usize > max_throws {
                    continue;
                }
                // We include indices wherever s has a one in its binary representation
                idx = 0;
                throw = Vec::with_capacity(max_throws-1);  // The subset
                while s > 0 {
                    if s & 1 == 1 {
                        throw.push(poss_throws_lst[idx]);
                    }
                    idx += 1;
                    s >>= 1;
                }
                if self.can_throw(&throw, &fallback, &self.player_to_play) {
                    poss_actions.push(tools::Action {
                        name: "throwcards".to_string(),
                        throw: throw.iter().map(|&x| *x).collect(),  // Dereference
                        ..Default::default()  // Throws no cards
                    });
                }
            }
        } else {
            unimplemented!();
        }

        return poss_actions;
    }

    fn execute_action(&mut self, action: tools::Action) {
        // Let the player to play perform action
        if action.name == "attack".to_string() {
            // Discard the played card
            let fallback = self.fallback_identities(&self.player_to_play);
            let hand = self.card_collection.hands.get_mut(&self.player_to_play).unwrap();
            // if let Some(index) = hand.iter().position(|val| (val.0, val.1) == action.card) {
            //     hand.swap_remove(index);
            // }
            // Remove a random card with that possible identity
            let mut idx_to_remove = None;
            let mut idx = 0;
            for card in hand.iter() {
                if card == &(-1, -1, false) {
                    if fallback.contains(&action.card) {
                        idx_to_remove = Some(idx);
                        break;
                    }
                } else {
                    if (card.0, card.1) == action.card {
                        idx_to_remove = Some(idx);
                        break;
                    }
                }
                idx += 1;
            }
            hand.swap_remove(idx_to_remove.expect("DID NOT FIND CARD IN HAND"));


            self.card_collection.non_public.remove(&action.card);
            // Set other values
            self.last_played_attacker = self.player_to_play;
            self.player_to_play = self.defender;
            self.current_action = tools::Action {
                name: "defend".to_string(),
                ..Default::default()
            };
            self.cards_to_defend.push(action.card)

        } else if action.name == "defend".to_string() {
            let card_defended = self.cards_to_defend.swap_remove(0);
            let fallback = self.fallback_identities(&self.player_to_play);
            // Discard the played card
            let hand = self.card_collection.hands.get_mut(&self.player_to_play).unwrap();
            // if let Some(index) = hand.iter().position(|val| (val.0, val.1) == action.card) {
            //     hand.swap_remove(index);
            // }
            // Remove a random card with that possible identity
            let mut idx_to_remove = None;
            let mut idx = 0;
            for card in hand.iter() {
                if card == &(-1, -1, false) {
                    if fallback.contains(&action.card) {
                        idx_to_remove = Some(idx);
                        break;
                    }
                } else {
                    if (card.0, card.1) == action.card {
                        idx_to_remove = Some(idx);
                        break;
                    }
                }
                idx += 1;
            }
            hand.swap_remove(idx_to_remove.expect("DID NOT FIND CARD IN HAND"));
            self.card_collection.non_public.remove(&action.card);
            self.pairs_finished.push((card_defended, action.card));
            if self.cards_to_defend.len() == 0 {
                // No more cards left to defend, switch to attacking
                self.player_to_play = self.attackers[self.current_attacker as usize];
                self.current_action = tools::Action {
                    name: "attack".to_string(),
                    ..Default::default()
                };
            }
        } else if action.name == "take".to_string() {
            self.current_action = tools::Action {
                name: "throwcards".to_string(),
                ..Default::default()
            };
            self.player_to_play = self.attackers[self.current_attacker as usize];
            self.attacker_to_start_throwing = self.current_attacker;
        } else if action.name == "throwcards".to_string() {
            // Discard all the cards that are thrown away
            let fallback = self.fallback_identities(&self.player_to_play);
            let hand = self.card_collection.hands.get_mut(&self.player_to_play).unwrap();
            // hand.retain(|x| ! action.throw.contains(&(x.0, x.1)));
            for card in &action.throw {
                self.card_collection.non_public.remove(&card);
                self.cards_to_defend.push(*card);
                // Remove a random card with that possible identity
                let mut idx_to_remove = None;
                let mut idx = 0;
                for hand_card in hand.iter() {
                    if hand_card == &(-1, -1, false) {
                        if fallback.contains(&card) {
                            idx_to_remove = Some(idx);
                            break;
                        }
                    } else {
                        if &(hand_card.0, hand_card.1) == card {
                            idx_to_remove = Some(idx);
                            break;
                        }
                    }
                    idx += 1;
                }
                hand.swap_remove(idx_to_remove.expect("DID NOT FIND CARD IN HAND"));
            }
            // Increase attacker and player to play
            self.current_attacker = (self.current_attacker + 1) % self.attackers.len() as u8;
            self.player_to_play = self.attackers[self.current_attacker as usize];
            // Check if everybody got the chance to throw their cards
            if self.player_to_play == self.attackers[self.attacker_to_start_throwing as usize] {
                let defender_cards = self.card_collection.hands.get_mut(&self.defender).unwrap();
                // Add cards and make them public
                // defender_cards.extend(&action.throw.iter().map(|x| (x.0, x.1, true)).collect::<Vec<(i8, i8, bool)>>());
                defender_cards.extend(&self.cards_to_defend.iter().map(|x| (x.0, x.1, true)).collect::<Vec<(i8, i8, bool)>>());
                defender_cards.extend(&self.pairs_finished.iter().map(|x| (x.0.0, x.0.1, true)).collect::<Vec<(i8, i8, bool)>>());
                defender_cards.extend(&self.pairs_finished.iter().map(|x| (x.1.0, x.1.1, true)).collect::<Vec<(i8, i8, bool)>>());
                for p in self.draw_order.clone() {
                    self.fill_hand(&p);
                }
                // The defender takes the cards, the new main attacker
                // sits to the left of the defender
                self.main_attacker = self.players[self.attackers[1 % self.attackers.len()] as usize].name.clone();
                self.new_trick()
            }
        } else if action.name == "passattack".to_string() {
            // The person passed on attacking, the next attacker may play
            self.current_attacker = (self.current_attacker + 1) % self.attackers.len() as u8;
            self.player_to_play = self.attackers[self.current_attacker as usize];
            // Check if we have an entire round of people that do not want to attack
            if self.player_to_play == self.last_played_attacker {
                // The defender defended successfully
                // Let everybody draw cards
                for p in self.draw_order.clone() {
                    self.fill_hand(&p);
                }
                assert_eq!(self.cards_to_defend, Vec::new());
                if self.print_info {
                    println!("The card pairs {:?} are removed from the game", self.pairs_finished)
                }
                // Initialize a new trick with the defender as main attacker
                self.main_attacker = self.players[self.defender as usize].name.clone();
                self.new_trick()
            }
        } else if action.name == "reflect".to_string() {
            // Discard the played card
            let fallback = self.fallback_identities(&self.player_to_play);
            let hand = self.card_collection.hands.get_mut(&self.player_to_play).unwrap();
            // if let Some(index) = hand.iter().position(|val| (val.0, val.1) == action.card) {
            //     hand.swap_remove(index);
            // }
            // Remove a random card with that possible identity
            let mut idx_to_remove = None;
            let mut idx = 0;
            for card in hand.iter() {
                if card == &(-1, -1, false) {
                    if fallback.contains(&action.card) {
                        idx_to_remove = Some(idx);
                        break;
                    }
                } else {
                    if (card.0, card.1) == action.card {
                        idx_to_remove = Some(idx);
                        break;
                    }
                }
                idx += 1;
            }
            hand.swap_remove(idx_to_remove.expect("DID NOT FIND CARD IN HAND"));
            self.card_collection.non_public.remove(&action.card);
            // The new defender sits left of the current defender, the main attacker
            // stays the same and the current cards need to be defended
            self.last_played_attacker = self.player_to_play;
            let num_old_attackers = self.attackers.len();
            let new_defender = self.attackers.remove(1 % num_old_attackers);
            self.attackers.insert(1 % num_old_attackers, self.defender);
            self.defender = new_defender;
            self.draw_order = self.attackers.clone();
            self.draw_order.push(self.defender);
            self.attackers.rotate_left(1);
            self.cards_to_defend.push(action.card);
            self.current_action = tools::Action {
                name: "defend".to_string(),
                ..Default::default()
            };
            self.player_to_play = self.defender;
        } else if action.name == "reflecttrump".to_string() {
            // Everyone now knows your trump card
            let fallback = self.fallback_identities(&self.player_to_play);
            let hand = self.card_collection.hands.get_mut(&self.player_to_play).unwrap();
            // Remove a random card with that possible identity
            let mut idx_to_remove = None;
            let mut idx = 0;
            for card in hand.iter() {
                if card == &(-1, -1, false) {
                    if fallback.contains(&action.card) {
                        idx_to_remove = Some(idx);
                        break;
                    }
                } else {
                    if (card.0, card.1) == action.card {
                        idx_to_remove = Some(idx);
                        break;
                    }
                }
                idx += 1;
            }
            let i = idx_to_remove.expect("DID NOT FIND CARD IN HAND");
            hand[i] = (action.card.0, action.card.1, true);
            // if let Some(index) = hand.iter().position(|val| (val.0, val.1) == action.card) {
            //     hand[index] = (hand[index].0, hand[index].1, true);
            // }
            self.card_collection.non_public.remove(&action.card);
            // This card loses its ability to reflect for the rest of this turn
            self.reflected_trumps.push(action.card);
            // The new defender sits left of the current defender, the main attacker
            // stays the same and the current cards need to be defended
            self.last_played_attacker = self.player_to_play;
            let num_old_attackers = self.attackers.len();
            let new_defender = self.attackers.remove(1 % num_old_attackers);
            self.attackers.insert(1 % num_old_attackers, self.defender);
            self.defender = new_defender;
            self.draw_order = self.attackers.clone();
            self.draw_order.push(self.defender);
            self.attackers.rotate_left(1);
            self.current_action = tools::Action {
                name: "defend".to_string(),
                ..Default::default()
            };
            self.player_to_play = self.defender;
        } else {
            unimplemented!();
        }
    }

    fn next(&mut self) {
        let action_to_play = self.choose_action();
        if self.print_info {
            println!("{:?}, {:?}", self.players[self.player_to_play as usize], action_to_play);
        }
        self.execute_action(action_to_play);
        // if self.print_info && self.computer_shuffle {
        //     println!("{:?}", self.card_collection);
        // }
    }

    fn get_random_action(&self) -> tools::Action {
        // Get a random action from all the allowed plays
        // Check the allowed plays of this player
        let allowed = self.allowed_plays();
        assert!(allowed.len() > 0);
        let idx = rand::thread_rng().gen_range(0..allowed.len());
        let action_to_play = allowed[idx].clone();
        return action_to_play;
    }

    fn choose_action(&mut self) -> tools::Action {
        // Choose an action from all allowed actions
        let player_idx = self.player_to_play;

        // Ask for the cards
        if self.computer_shuffle {
            // The computer shuffles so all need to know their cards
            self.make_cards_known(&self.player_to_play.clone());
        } else {
            // Ask maintainer for card information of person
            if self.players[self.player_to_play as usize]._type != "HUMAN".to_string() {
                // All non-human players need to know their cards to form a decision
                self.make_cards_known(&self.player_to_play.clone());
            }
        }

        if self.players[player_idx as usize]._type == "RANDOM".to_string() {
            return self.get_random_action();
        } else if self.players[player_idx as usize]._type == "DeterminizedMCTS".to_string() {
            // Check the allowed plays of this player
            let allowed = self.allowed_plays();

            assert!(allowed.len() > 0);

            // Check if there is only one allowed play
            if allowed.len() == 1 {
                return allowed[0].clone()
            }

            // The cards in all the other players hands are unknown to us
            // So we must reset them (since otherwise they are known) to them
            let mut copied_state = self.clone();
            // println!("COPIED!");
            for p in 0..self.players.len() {
                if p as u8 != player_idx {
                    let hand = copied_state.card_collection.hands.get_mut(&(p as u8)).unwrap();
                    for card in hand {
                        if card.2 == false {  // Only private cards are reset
                            *card = (-1, -1, false);
                        }
                    }
                }
            }


            let threads: u32 = self.players[self.player_to_play as usize].threads;
            let deals = self.players[self.player_to_play as usize].deals;
            let num_rollouts = self.players[self.player_to_play as usize].rollouts;

            let deal_per_thread = deals/threads;
            let extra_deals = deals - deal_per_thread*threads;

            let mut handles = Vec::new();

            for t in 0..threads {
                let mut deals_now = deal_per_thread;
                if t < extra_deals {
                    deals_now += 1;
                }
                // println!("{}, {}", t, deals_now);
                let new_copied_state = copied_state.clone();
                let handle = thread::spawn(move || {
                    let mut ratings: HashMap<tools::Action, [u32; 2]> = HashMap::new();
                    for _deal in 0..deals_now {
                        // We do not want to change the current game state
                        let mut copied = new_copied_state.clone();
                        // Perform a random deal of all cards
                        let mut to_divide = copied.fallback_identities(&copied.player_to_play);
                        let mut idx;
                        let mut info;
                        for card_idx in 0..copied.deck.len() {
                            if ! copied.deck[card_idx].is_public {
                                idx = rand::thread_rng().gen_range(0..to_divide.len());
                                info = to_divide.swap_remove(idx);
                                copied.deck[card_idx].suit = info.0;
                                copied.deck[card_idx].value = info.1;
                                copied.deck[card_idx].is_unknown = false;
                                copied.deck[card_idx].is_private = false;
                                copied.deck[card_idx].is_public = true;
                            }
                        }
                        for p in 0..copied.players.len() {
                            let hand = copied.card_collection.hands.get_mut(&(p as u8)).unwrap();
                            for card in hand {
                                if card == &(-1, -1, false) {
                                    idx = rand::thread_rng().gen_range(0..to_divide.len());
                                    info = to_divide.swap_remove(idx);
                                    *card = (info.0, info.1, true);
                                }
                            }
                        }
                        assert_eq!(to_divide.len(), 0);
                        copied.print_info = false;

                        let mut tree = MCTree {
                            player_name: copied.players[player_idx as usize].name.clone(),
                            expl_const: copied.players[player_idx as usize].expl_const,
                            ..Default::default()
                        };
                        let mut dct = tree.do_rollouts(copied, num_rollouts);
                        for (action, scores) in dct.drain() {
                            if ratings.contains_key(&action) {
                                let vals = ratings.get_mut(&action).unwrap();
                                vals[0] += scores.0;
                                vals[1] += scores.1;
                            } else {
                                ratings.insert(action, [scores.0, scores.1]);
                            }
                        }
                    }
                    return ratings;
                });
                handles.push(handle);
            }

            let mut total_ratings: HashMap<tools::Action, [u32; 2]> = HashMap::new();
            for handle in handles {
                let mut dct = handle.join().unwrap();
                for (action, scores) in dct.drain() {
                    // println!("{:?}, {}, {}", action, scores.0, scores.1);
                    if total_ratings.contains_key(&action) {
                        let vals = total_ratings.get_mut(&action).unwrap();
                        vals[0] += scores[0];
                        vals[1] += scores[1];
                    } else {
                        total_ratings.insert(action, [scores[0], scores[1]]);
                    }
                }
            }

            let mut highest_n = -1.0;  // 0 is also a valid score
            let mut best_action = None;
            let mut num_lines = 0;
            if self.players[self.player_to_play as usize].confirm {
                println!("");
                num_lines += 1;
            }
            for (action, scores) in total_ratings.drain() {
                let winning_percentage = (scores[0] as f64) / (scores[1] as f64);
                if self.players[self.player_to_play as usize].confirm {
                    println!("{:?}, win_rate={:.2}%, W={}, N={}", action, winning_percentage*100.0, scores[0], scores[1]);
                    num_lines += 1;
                }
                if winning_percentage > highest_n {
                    highest_n = winning_percentage;
                // if scores[1] > highest_n {
                //     highest_n = scores[1];
                    best_action = Some(action);
                }
            }
            let action_to_play = best_action.unwrap();
            if self.players[self.player_to_play as usize].confirm {
                println!("GOING FOR ACTION {:?}", action_to_play);
                num_lines += 1;
            }

            if self.players[self.player_to_play as usize].confirm == true {
                // We have to confirm this action
                let mut s=String::new();
                print!("Confirm [y/n]: ");
                let _=stdout().flush();
                stdin().read_line(&mut s).expect("Did not enter a correct string");
                if let Some('\n')=s.chars().next_back() {
                    s.pop();
                }
                if let Some('\r')=s.chars().next_back() {
                    s.pop();
                }
                if s != "y" {
                    println!("Not confirmed, choose an action yourself.");
                    // Clear output
                    return self.let_human_choose(num_lines+2);
                } else {
                    for _ in 0..num_lines+1 {
                        print!("\x1b[F\x1b[J");
                    }
                }
            }
            return action_to_play;
        } else if self.players[player_idx as usize]._type == "HUMAN".to_string() {
            return self.let_human_choose(0);
        } else {
            unimplemented!();
        }
    }

    fn let_human_choose(&self, lines_to_remove: u32) -> tools::Action {
        let allowed = self.allowed_plays();
        if allowed.len() == 1 {
            return allowed[0].clone();
        }
        let mut tot_lines = lines_to_remove;
        println!("");
        println!("{:?} is to play", self.players[self.player_to_play as usize]);
        tot_lines += 2;
        let mut idx: usize;
        let mut i = 0;
        for action in &allowed {
            println!("{:3}) {:?}", i, action);
            i += 1;
        }
        tot_lines += i;
        loop {
            let mut s=String::new();
            print!("INDEX OF THE ACTION TO PLAY/PLAYED: ");
            tot_lines += 1;
            let _=stdout().flush();
            stdin().read_line(&mut s).expect("Did not enter a correct string");
            if let Some('\n')=s.chars().next_back() {
                s.pop();
            }
            if let Some('\r')=s.chars().next_back() {
                s.pop();
            }
            idx = match s.parse() {
                Ok(num) => num,
                Err(_) => continue,
            };
            if idx >= allowed.len() {
                println!("NOT ALLOWED, TRY AGAIN");
                tot_lines += 1;
                continue;
            }
            break;
        }
        // Clear output
        for _ in 0..tot_lines {
            print!("\x1b[F\x1b[J");
        }
        return allowed[idx].clone();
    }
}


#[derive(Clone)]
struct MCNode {
    w: u32,
    n: u32,
    is_end_state: bool,
    game_state: GameVals,
    is_explored: bool,
    children: HashMap<tools::Action, MCNode>,
    unexplored_children: Vec<tools::Action>,
}

impl Default for MCNode {
    fn default() -> Self {
        MCNode {
            w: 0,
            n: 0,
            is_end_state: false,
            game_state: Default::default(),
            is_explored: false,
            children: HashMap::new(),
            unexplored_children: Vec::new(),
        }
    }
}

trait WorkNode {
    fn uct_select(&self, expl_const: f64) -> tools::Action;
}

impl WorkNode for MCNode {
    fn uct_select(&self, expl_const: f64) -> tools::Action {
        let mut kids = Vec::new();
        for (action_played, child) in &self.children {
            let (n, w) = (child.n, child.w);
            if n > 0 {
                kids.push((action_played, n, w))
            }
        }

        assert!(self.n > 0);
        assert!(kids.len() > 0);

        let mult = expl_const * (self.n as f64).ln().sqrt();
        let mut best: f64 = f64::MIN;  // Best score
        let mut best_kid = None;
        for kid in kids {
            let score = (kid.2 as f64) / (kid.1 as f64) + mult / (kid.1 as f64).sqrt();
            if score > best {
                best_kid = Some(kid.0);
                best = score;
            }
        }
        // println!("{:?}, {}", best_kid, best);
        // let action = self.children.keys().nth(0).unwrap().clone();
        return best_kid.unwrap().clone();
    }
}

struct MCTree {
    mctsnode: MCNode,
    path: Vec<tools::Action>,
    expl_const: f64,
    player_name: String,
}

impl Default for MCTree {
    fn default() -> Self {
        MCTree {
            mctsnode: Default::default(),
            expl_const: 0.8,
            player_name: "main".to_string(),
            path: Vec::new(),
        }
    }
}

trait MCTS {
    fn do_rollouts(&mut self, game_state: GameVals, rollouts: u32) -> HashMap<tools::Action, (u32, u32)>;
    fn select_expand_simulate(&mut self) -> String;
    fn backpropagate(&mut self, loser: String);
}

impl MCTS for MCTree {
    fn do_rollouts(&mut self, game_state: GameVals, rollouts: u32) -> HashMap<tools::Action, (u32, u32)> {
        // Define the root node
        self.mctsnode = MCNode {
            is_end_state: game_state.is_end_state,
            game_state: game_state,
            ..Default::default()
        };
        // Perform the rollouts/traversals
        for r in 0..rollouts {
            print!("Doing rollout {} for {}\r", r, self.player_name);
            let loser = self.select_expand_simulate();
            self.backpropagate(loser);
        }
        let mut dct = HashMap::new();
        for (action_played, child) in self.mctsnode.children.drain() {
            dct.insert(action_played, (child.w, child.n));
            // println!("{:?}, {}, {}", action_played, child.n, child.w);
        }
        return dct;
    }

    fn select_expand_simulate(&mut self) -> String {
        let mut mctsnode = &mut self.mctsnode;
        // Final leaf node to expand
        let leaf_node;
        let mut child;
        // Check if root
        loop {
            if ! mctsnode.is_explored {
                leaf_node = mctsnode;
                break;
            }

            if mctsnode.is_end_state {
                leaf_node = mctsnode;
                break;
            }

            // We go to an unexplored child
            if mctsnode.unexplored_children.len() > 0 {
                // We explore a child by finding its game state
                let action = mctsnode.unexplored_children.pop().unwrap();
                self.path.push(action.clone());
                let mut copied = mctsnode.game_state.clone();
                copied.execute_action(action.clone());

                child = MCNode {
                    is_end_state: copied.is_end_state,
                    game_state: copied,
                    ..Default::default()
                };
                mctsnode.children.insert(action, child.clone());
                leaf_node = &mut child;
                break;
            } else {
                let action = mctsnode.uct_select(self.expl_const);
                self.path.push(action.clone());
                let node = mctsnode.children.get_mut(&action).unwrap();
                mctsnode = node;
            }
        }

        if leaf_node.is_end_state {
            return leaf_node.game_state.loser.clone();
        }

        // Expand the leaf node
        let game = &leaf_node.game_state;
        // Check which actions we're allowed to do
        let allowed = game.allowed_plays();
        // Initialize the children of the leaf node
        leaf_node.unexplored_children.extend(allowed);
        leaf_node.is_explored = true;

        // Simulate, i.e. player a raudnom game until an end state is reached
        let mut copied = leaf_node.game_state.clone();
        while ! copied.is_end_state {
            // Execute a random allowed action
            let mut allowed = copied.allowed_plays();
            let idx = rand::thread_rng().gen_range(0..allowed.len());
            let action = allowed.swap_remove(idx);
            copied.execute_action(action);
        }
        return copied.loser.clone();
    }

    fn backpropagate(&mut self, loser: String) {
        let mut mctsnode = &mut self.mctsnode;
        mctsnode.n += 1;  // Only the n is needed of the root
        for action in self.path.drain(..) {
            let current_player = &mctsnode.game_state.players[mctsnode.game_state.player_to_play as usize].name;
            mctsnode = mctsnode.children.get_mut(&action).unwrap();
            mctsnode.n += 1;
            if *current_player != loser {
                mctsnode.w += 1;
            }
        }
    }
}

fn input_card(possible_cards: Vec<(i8, i8)>) -> (usize, usize) {
    // Let someone enter the card as input, this outputs the suit, value
    let mut suit: usize;
    let mut value: usize;
    let mut tot_lines = 0;
    loop {
        let mut s=String::new();
        print!("Suit of the card [♣♠♥♦]: ");
        tot_lines += 1;
        let _=stdout().flush();
        stdin().read_line(&mut s).expect("Did not enter a correct string");
        if let Some('\n')=s.chars().next_back() {
            s.pop();
        }
        if let Some('\r')=s.chars().next_back() {
            s.pop();
        }
        suit = match s.parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        let mut s=String::new();
        print!("Value of the card [6789*JQKA]: ");
        tot_lines += 1;
        let _=stdout().flush();
        stdin().read_line(&mut s).expect("Did not enter a correct string");
        if let Some('\n')=s.chars().next_back() {
            s.pop();
        }
        if let Some('\r')=s.chars().next_back() {
            s.pop();
        }
        value = match s.parse() {
            Ok(num) => num,
            Err(_) => continue,
        };
        if suit > 3 || value > 8 {
            continue;
        }
        if ! possible_cards.contains(&(suit as i8, value as i8)) {
            println!("Try again, the options are");
            tot_lines += 1;
            let mut line = 0;
            for opt in &possible_cards {
                // Print in 5 columns
                print!("{:?} ", opt);
                line += 1;
                if line % 5 == 0 {
                    println!("");
                    tot_lines += 1;
                }
            }
            println!("");
            tot_lines += 1;
            continue;
        }
        break;
    }
    // Clear output
    for _ in 0..tot_lines {
        print!("\x1b[F\x1b[J");
    }

    return (suit, value);
}

fn rate_players() {
    // We rate players via head-to-head games
    // A random player has an elo of 1000
    // Note that this random player differs from the python version
    // as the defending moves are not given weights.
    let mut tot_won: u32 = 0;
    let games_to_play = 10000;
    let rating_others = 1000.0;  // The rating of player 2
    let uncer_others = 0.0;  // Uncertainty in the rating
    // let rating_others = 1495.01;  // The rating of player 2
    // let uncer_others = 7.64;  // Uncertainty in the rating

    for game_num in 0..games_to_play {
        let mut players = Vec::with_capacity(6);
        // idx must be sorted
        players.push(tools::Player {
            idx: 0,
            name: "Player1".to_string(),
            _type: "DeterminizedMCTS".to_string(),
            rollouts: 10,
            threads: 6,
            deals: 100,  // Best to have this a multiple of threads
            expl_const: 0.8,
            // _type: "RANDOM".to_string(),
            ..Default::default()
        });
        players.push(tools::Player {
            idx: 1,
            name: "Player2".to_string(),
            // _type: "DeterminizedMCTS".to_string(),
            // rollouts: 10,
            // threads: 3,
            // deals: 3,  // Best to have this a multiple of threads
            // expl_const: 0.8,
            _type: "RANDOM".to_string(),
            ..Default::default()
        });

        let player_names = ["Player1".to_string(), "Player2".to_string()];
        let starting_player = player_names[rand::thread_rng().gen_range(0..2)].clone();
        let mut game = GameVals {
            players: players,
            // computer_shuffle: false,
            main_attacker: starting_player,
            print_info: false,
            ..Default::default()
        };

        game.initialize();
        while ! game.is_end_state {
            game.next();
        }
        if game.loser != "Player1".to_string() {
            tot_won += 1;
        }
        let observed_e_a: f64 = (tot_won as f64) / (game_num as f64);
        let r_a = rating_others - 400.0 * ((1_f64/observed_e_a - 1_f64).log10());
        let uncer = (uncer_others*uncer_others + (400_f64*400_f64 / 10_f64.ln() / 10_f64.ln() / (observed_e_a * (1_f64-observed_e_a)) / (game_num as f64))).sqrt();
        println!("GAME {} IS LOST BY {}, Player 1 is rated {:.2} ± {:.2}", game_num, game.loser, r_a, uncer);
    }

    // 1) Random players are set to have an elo rating of 1000 ± 0
    // 2) Letting play Random vs (1) for 1000000 games
    //              gives elo 1000.11 ± 0.35 (finished in 132.30s)
    // 3) Letting play Random(always starting player) vs (1) for 1000000 games
    //              gives elo 1000.13 ± 0.35 (finished in 131.99s)
    // 4) Letting play DeterminizedMCTS(deals=1, rollouts=5, expl=0.8) vs (1) for 10000 games
    //              gives elo 1214.83 ± 4.16 (finished in 261.91s)
    // 5) Letting play DeterminizedMCTS(deals=3, rollouts=5, expl=0.8) vs (1) for 10000 games
    //              gives elo 1360.53 ± 5.52 (finished in 752.08s)
    // 6) Letting play DeterminizedMCTS(deals=3, rollouts=10, expl=0.8) vs (1) for 10000 games
    //              gives elo 1495.01 ± 7.64 (finished in 1927.09s)
    // 7) Letting play DeterminizedMCTS(deals=5, rollouts=20, expl=0.8) vs (6) for 6333 games
    //              gives elo 1640.71 ± 9.00 (finished in 4497.28s)
    // 8) Letting play DeterminizedMCTS(deals=10, rollouts=50, expl=0.8) vs (7) for 2000 games
    //              gives elo 1803.42 ± 12.47 (finished in 910.38s)
    // 9) Letting play DeterminizedMCTS(deals=20, rollouts=100, expl=0.8) vs (7) for 2000 games
    //              gives elo 1943.41 ± 14.14 (finished in 1716.29s)
    // 10) Letting play DeterminizedMCTS(deals=25, rollouts=500, expl=0.8) vs (8) for 777 games
    //              gives elo 2069.67 ± 20.53 (finished in 2387.81s)
    // 11) Letting play DeterminizedMCTS(deals=100, rollouts=10, expl=0.8) vs (6) for 5171 games
    //              gives elo 1826.20 ± 10.50 (finished in 4597.55s)
    // 12) Letting play DeterminizedMCTS(deals=56, rollouts=5000, expl=0.8) vs (10) for 266 games
    //              gives elo 2197.98 ± 30.66 (finished in 12037.64s)
    // 13) Letting play DeterminizedMCTS(deals=56, rollouts=5000, expl=0.7) vs (10) for 88 games
    //              gives elo 2240.06 ± 46.37 (finished in 4249.59s)
    //
}

fn play_game() {
    let mut players = Vec::with_capacity(6);
    // This simulates a durak game
    // Note, the index of the players must be 0,1,2,...
    // If computer_shuffle is false you must specify the cards yourself.
    //          This functionality was added for in-game help.
    // If confirm is set to true in the player its moves must be confirmed (and can be changed).
    // The actions of all HUMAN players must be specified, the cards of the player are not needed
    //          as input.
    // The starting/main attacker must be specified below.

    players.push(tools::Player {
        idx: 0,
        name: "Player1".to_string(),
        _type: "DeterminizedMCTS".to_string(),
        rollouts: 500,
        threads: 3,
        deals: 12,  // Best to have this a multiple of threads
        expl_const: 0.8,
        // confirm: true,  // If we have to confirm this players actions
        // _type: "RANDOM".to_string(),
        ..Default::default()
    });
    players.push(tools::Player {
        idx: 1,
        name: "Player2".to_string(),
        _type: "RANDOM".to_string(),
        ..Default::default()
    });
    // players.push(tools::Player {
    //     idx: 2,
    //     name: "Player3".to_string(),
    //     _type: "HUMAN".to_string(),
    //     ..Default::default()
    // });
    // players.push(tools::Player {
    //     idx: 3,
    //     name: "Player3".to_string(),
    //     _type: "RANDOM".to_string(),
    //     ..Default::default()
    // });

    let mut game = GameVals {
        players: players,
        // computer_shuffle: false,
        main_attacker: "Player1".to_string(),
        ..Default::default()
    };

    game.initialize();
    while ! game.is_end_state {
        game.next();
    }
    println!("LOSER OF THE GAME = {:?}", game.loser);

}



// fn main() {
//     rate_players();
// }
fn main() {
    play_game();
}
