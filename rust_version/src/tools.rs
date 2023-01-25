use std::collections::HashSet;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub struct Card {
    pub suit: i8,
    pub value: i8,
    pub trump_suit: i8,
    pub is_public: bool,
    pub is_private: bool,
    pub is_unknown: bool,
}

impl Default for Card {
    fn default() -> Card {
        Card {
            suit: -1,
            value: -1,
            trump_suit: -1,
            is_public: false,
            is_private: false,
            is_unknown: true,
        }
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_unknown {
            return write!(f, "??")
        }
        let mut suits = "♣♠♥♦".chars();
        let mut values = "6789*JQKA".chars();
        return write!(f, "{}{}", suits.nth(self.suit as usize).unwrap(), values.nth(self.value as usize).unwrap());
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Just display the card (with permissions)
        if self.is_unknown {
            return write!(f, "?{}", self)
        } else if self.is_private {
            return write!(f, "P{}", self)
        } else {
            return write!(f, "A{}", self)
        }
    }
}


#[derive(Clone)]
pub struct CardCollection {
    // Collection of all suit, value pairs in the game
    pub unknown: Vec<(i8, i8)>,
    pub non_public: HashSet<(i8, i8)>,
    // pub public: Vec<(i8, i8)>,
    pub hands: HashMap<u8, Vec<(i8, i8, bool)>>,
    pub trump_suit: i8,
}


impl Default for CardCollection {
    fn default() -> CardCollection {
        let mut all_cards = Vec::new();
        for suit in 0..4 {
            for value in 0..9 {
                all_cards.push((suit as i8, value as i8));
            }
        }
        CardCollection {
            unknown: all_cards.to_vec(),
            non_public: all_cards.iter().map(|x| *x).collect(),
            hands: HashMap::with_capacity(36),
            trump_suit: -1,
        }
    }
}

impl std::fmt::Debug for CardCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Display the groups of cards
        let suits = "♣♠♥♦".chars();
        let values = "6789*JQKA".chars();
        let mut ans: String = "".to_string();
        ans += "CardCollection[";
        ans += &self.unknown.len().to_string();
        ans += " unknown + ";
        for p in 0..self.hands.len() {
            ans += "Player_";
            ans += &p.to_string();
            ans += "(";
            for card in &self.hands[&(p as u8)] {
                if card == &(-1, -1, false) {
                    ans += "?? ";
                    continue
                }
                if card.2 {
                    ans += "A"  // public
                } else {
                    ans += "P"  // private
                }
                ans += &suits.clone().nth(card.0 as usize).unwrap().to_string();
                ans += &values.clone().nth(card.1 as usize).unwrap().to_string();
                ans += " ";
            }
            ans += ")";
        }
        ans += "]";

        write!(f, "{}", ans)
    }
}

#[derive(Clone)]
pub struct Player {
    pub idx: u8,
    pub name: String,
    pub _type: String,
    pub rollouts: u32,
    pub threads: u32,
    pub expl_const: f64,
    pub deals: u32,
    pub confirm: bool,
}

impl Default for Player {
    fn default() -> Player {
        Player {
            idx: 0,
            name: "??".to_string(),
            _type: "RANDOM".to_string(),
            rollouts: 1000,
            deals: 10,
            expl_const: 0.8,
            threads: 2,
            confirm: false,
        }
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(f, "Player({}, type={})", self.name, self._type)
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Action {
    pub name: String,
    pub card: (i8, i8),
    pub throw: Vec<(i8, i8)>,
}

impl Default for Action {
    fn default() -> Action {
        Action {
            name: "attack".to_string(),
            card: (-1, -1),
            throw: Vec::with_capacity(27),
        }
    }
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let suits = "♣♠♥♦".chars();
        let values = "6789*JQKA".chars();

        if vec!["attack".to_string(), "defend".to_string(), "reflect".to_string(), "reflecttrump".to_string()].contains(&self.name) {
            return write!(f, "Action({}, card={}{})", self.name, &suits.clone().nth(self.card.0 as usize).unwrap().to_string(), &values.clone().nth(self.card.1 as usize).unwrap().to_string());
        } else if vec!["passattack".to_string(), "take".to_string()].contains(&self.name) {
            return write!(f, "Action({})", self.name)
        } else if "throwcards".to_string() == self.name {
            return write!(f, "Action({}, cards={:?})", self.name, self.throw)
        } else {
            return write!(f, "Action({}, card={:?})", self.name, self.card)
        }
    }
}
