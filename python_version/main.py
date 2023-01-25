from collections import defaultdict
from itertools import combinations

from tools import Card
from tools import choose_random
from players import Random, Human, ISMCTSFPV, DeterminizedMCTS, ISMCTS


class GameTree:
    """The main tree for the game durak

    Args:
        - players:          The players in the game as classes
        - computer_shuffle: If set to True, the computer shuffles and the
                                game is virtual, if set to False, you have
                                to shuffle and draw the cards irl and tell
                                it to the computer.
        - main_attacker:    The name of the starting attacker
    """
    is_end_state = False
    loser = None  # The loser/durak of the game once known
    print_info = True

    def __init__(self, players, computer_shuffle, main_attacker, do_init=True, print_info=True):
        if not do_init:  # Terminate the init (for make_deepcopy)
            return

        self.players = players
        assert 2 <= len(self.players) <= 6  # No fun to cards run out
        self.computer_shuffle = computer_shuffle
        self.print_info = print_info

        # Initialize the deck
        self.deck = [Card() for _ in range(36)]
        # Initialize a collection of all cards
        self.all_cards = {(suit, value) for suit in range(4) for value in range(9)}
        # Initialize an array with all cards, stored (nothing is
        # removed or added to this list), it functions as a collective
        # of which cards are known/private/public.
        self.card_collection = self.deck.copy()
        # Keep track of all the actions played this game
        self.history = []

        # Initialize the last/bottom card of the deck
        if computer_shuffle:
            unknown = self.get_unknown_cards()
            # At this point, unknown should equal all the (suit, value) pairs
            assert self.all_cards == unknown
            # Initialize the card as random card from all cards
            self.deck[-1].from_suit_value(*choose_random(unknown))
            self.deck[-1].is_public = True
        else:
            print('Specify the suit and value of the bottom card')
            self.deck[-1].from_input(self.all_cards)
            self.deck[-1].is_public = True

        # Check if the bottom card is an ace
        if self.deck[-1].value == 8:
            # If so, redeal
            if self.print_info:
                print('There was an ace on the bottom, redealing...')
            GameTree.__init__(self, players, computer_shuffle, main_attacker, print_info=self.print_info)
            return None

        # Display the bottom card
        if self.print_info:
            print(f'The bottom card is {str(self.deck[-1])[1:]}')

        # Set the trump suit of all cards to the suit of the bottom card
        for card in self.deck:
            card.trump_suit = self.deck[-1].suit

        # We initialize the hand of each player
        for p in players:
            # The suit and value of the card is not necessary at this point
            # only if the player is not Human the suit and value of the card
            # must be added whenever all possible actions are listed.
            self.deck = p.fill_hand(self.deck)

        # Initialize a new trick
        self.new_trick(main_attacker)


    def get_unknown_cards(self):
        """Returns the (suit, value) pairs of all unknown cards"""
        # remove = set()
        # for card in self.card_collection:
        #     if not card.is_unknown:
        #         remove.add((card.suit, card.value))
        remove = {(c.suit, c.value) for c in self.card_collection if not c.is_unknown}
        return self.all_cards - remove

    def get_non_public_cards(self):
        """Returns the (suit, value) pairs of all unknown cards + private cards"""
        # remove = set()
        # for card in self.card_collection:
        #     if card.is_public:
        #         remove.add((card.suit, card.value))
        remove = {(c.suit, c.value) for c in self.card_collection if c.is_public}
        return self.all_cards - remove


    def new_trick(self, main_attacker):
        """We initialize a new trick with main_attacker as starting player"""
        # We search for the main attacker in all players
        people = len(self.players)
        for idx, player in enumerate(self.players):
            if player.name == main_attacker:
                break
        else:
            raise 'Player name not known error'

        # We initialize the players starting from the main attacker
        # A person is still in the game whenever his hand is not empty or if
        # he can draw card.
        self.attackers = [person for i in range(idx, idx+people)
                            if (len((person := self.players[i%people]).hand) > 0 or
                                len(self.deck) > 0)]

        # Check if the game has ended
        if len(self.attackers) == 0:
            # There are no attackers, the last card got defended
            # and no one has any cards left, the last defender lost.
            self.is_end_state = True
            self.loser = self.defender
        elif len(self.attackers) == 1:
            # There is only one person left in the game, the loser
            self.is_end_state = True
            self.loser = self.attackers[0]
        else:
            # The game may continue
            self.defender = self.attackers.pop(1)
            # The main attacker may start off as the player to play
            self.current_attacker = 0
            self.player_to_play = self.attackers[0]
            # People must draw cards according to the draw order
            self.draw_order = self.attackers + [self.defender]
            # The person to the left of the defender if he takes
            self.attacker_to_start_throwing = None  # To check if all have thrown
            # To check if all attackers passed
            self.last_played_attacker = None
            # All the trumps that were used to reflect this turn (they lost their life)
            self.reflected_trumps = []

        # The action to perform
        self.current_action = 'Attack'
        # Succesfully defended cards as (attack, defend) pairs
        self.pairs_finished = []
        self.cards_to_defend = []


    def allowed_plays(self):
        # We enumerate the possible actions of
        player = self.player_to_play
        # who will perform
        action = self.current_action

        poss_actions = []
        if action == 'Attack':
            # The player must attack the defender
            attacker = self.attackers[self.current_attacker]
            assert attacker == player

            # List the possible plays of the player
            poss_plays = player.possible_card_plays(self.get_non_public_cards())

            if len(self.pairs_finished) > 0:
                # There is a pair on the table thus you can pass on attacking
                poss_actions.append(('PassAttack', None))
                # If you do not pass, you must play cards with the same
                # value as those that lie on the table.
                values_on_table = {card.value for pair in self.pairs_finished for card in pair}
                poss_plays = {c for c in poss_plays if c[1] in values_on_table}

            # Check if you are allowed to make another pile
            if len(self.defender.hand) > 0:
                # Iterate through the cards you can play
                for suit, value in poss_plays:
                    poss_actions.append(('Attack', (suit, value)))

            # if self.print_info:
            #     print(f"Person {player} attacks with one of [{' '.join('♣♠♥♦'[i] + '6789*JQKA'[j] for i, j in sorted(poss_plays))}]")
        elif action == 'Defend':
            # We need to defend the first card from the single ones
            to_defend = self.cards_to_defend[0]

            # We iterate through each card in the hand to see if we can use
            # it to defend the to_defend card.
            play_options = defaultdict(int)
            for card in player.hand:
                # Find the possible identities of this card
                if card.is_unknown:
                    identities = self.get_non_public_cards() - \
                            {(c.suit, c.value) for c in player.hand if not c.is_unknown}
                else:
                    identities = {(card.suit, card.value)}

                # Check if we can play that identity
                reflect = []
                defend = []
                for suit, value in identities:
                    ### Reflecting
                    # Only if there are no finished pairs you can reflect
                    if len(self.pairs_finished) == 0:
                        # Check if you are allowed to make another pile with reflecting
                        # The hypothetical new defender becomes
                        new_defender = self.attackers[1 % len(self.attackers)]
                        # The new defender must be able to defend all cards (if he wants)
                        # with the amount of cards in his hand.
                        max_new_piles = len(new_defender.hand) - len(self.cards_to_defend)
                        # If we reflect by playing the card we create another pile
                        if max_new_piles >= 1:
                            # Check if you can reflect the to_defend card with this card
                            if value == to_defend.value:
                                reflect.append(('Reflect', (suit, value)))
                        # If, however we reflect by showing a trump we
                        # do not have to create another pile
                        if max_new_piles >= 0:
                            # Check if you can reflect the to_defend card by showing your trump
                            if value == to_defend.value and suit == to_defend.trump_suit:
                                # Check if we already reflected with this trump this trick
                                if (suit, value) not in self.reflected_trumps:
                                    reflect.append(('ReflectTrump', (suit, value)))

                    ### Defending
                    # Check if you can defend the to_defend card with this card
                    if suit == to_defend.trump_suit and not to_defend.is_trump():
                        # You can always play a trump on a non-trump card to win
                        defend.append(('Defend', (suit, value)))
                    if suit == to_defend.suit:
                        # If the cards are of the same suit, you need to play a higher card
                        if value > to_defend.value:
                            defend.append(('Defend', (suit, value)))

                # Add all the options together to the possible actions.
                # Weights are added to prevent from overreflecting and too good cards
                # from another perspetive
                for action in defend:
                    play_options[action] += 1 / len(defend)
                for action in reflect:
                    play_options[action] += 1 / len(reflect)

            # Restructure the playing options to normal actions formats
            for key, weight in play_options.items():
                poss_actions.append(key + (weight,))

            # As the defender you can always take up the cards
            poss_actions.append(('Take', None, 1/2))

            # if self.print_info:
            #     print(f"Person {player} defends")
        elif action == 'ThrowCards':
            # The attacker gets the chance to throw cards on the pile
            # List the possible plays of the player
            poss_throws = player.possible_card_plays(self.get_non_public_cards())

            # You can only throw cards with the same value as those on the table
            values_on_table = {card.value for pair in self.pairs_finished for card in pair}
            values_on_table.update({card.value for card in self.cards_to_defend})
            poss_throws = {c for c in poss_throws if c[1] in values_on_table}
            # The amount of cards you can throw equals the number of cards in the hand
            # of the defender (originally) minus the amount of piles.
            # Hence, the number of current cards in the hand minus the amount of cards to defend.
            available_throws = len(self.defender.hand) - len(self.cards_to_defend)
            # If 0 cards are thrown
            poss_actions.append(('ThrowCards', (None,)))
            # If more than 0 cards are thrown
            max_throws = min(available_throws, len(poss_throws), len(player.hand))
            if max_throws > 0:
                fallback_identities = self.get_non_public_cards()
            for throw in range(1, max_throws + 1):
                # Any combination of throws are possible
                for option in combinations(poss_throws, r=throw):
                    if player.can_throw(fallback_identities, list(option)):
                        poss_actions.append(('ThrowCards', option))
                        # print(player, 'THROWING...', tmp)
            # if self.print_info:
            #     print(f"Person {player} can throw")
        else:
            raise ValueError(f'Action {action} not known')

        if len(poss_actions) == 0:
            # No actions are allowed
            raise BaseException('No choice of actions')

        return poss_actions

    def get_id(self):
        return hash(tuple(self.history))

    def execute_action(self, action):
        # Add action to history
        self.history.append(action)

        if action[0] == 'Attack':
            suit, value = action[1]
            # We need to go from a (suit, value) pair to the card
            card_played = self.player_to_play.discard_card(self, suit, value)
            # Set other values
            self.last_played_attacker = self.player_to_play
            self.player_to_play = self.defender
            self.current_action = 'Defend'
            self.cards_to_defend.append(card_played)
        elif action[0] == 'Defend':
            card_defended = self.cards_to_defend.pop(0)
            suit, value = action[1]
            ### We defend the card_defended card with (suit, value)
            card_played = self.player_to_play.discard_card(self, suit, value)
            self.pairs_finished += [(card_defended, card_played)]
            if len(self.cards_to_defend) == 0:
                # There are no more cards left to defend, switch to attacking again
                self.player_to_play = self.attackers[self.current_attacker]
                self.current_action = 'Attack'
        elif action[0] == 'Take':
            self.current_action = 'ThrowCards'
            self.player_to_play = self.attackers[self.current_attacker]
            self.attacker_to_start_throwing = self.current_attacker
        elif action[0] == 'ThrowCards':
            if action[1][0] is not None:
                # We must throw some cards
                cards_to_throw = action[1]
                for suit, value in cards_to_throw:
                    # NOTE: is discarding in a certain order necessary??
                    card_played = self.player_to_play.discard_card(self, suit, value)
                    self.cards_to_defend.append(card_played)
            # Increase attacker and player to play
            self.current_attacker = (self.current_attacker + 1) % len(self.attackers)
            self.player_to_play = self.attackers[self.current_attacker]

            # Check if everybody got the chance to throw their cards
            if self.player_to_play == self.attackers[self.attacker_to_start_throwing]:
                cards_on_table = [card for pair in self.pairs_finished for card in pair]
                cards_on_table += self.cards_to_defend
                self.defender.hand += cards_on_table
                for p in self.draw_order:
                    self.deck = p.fill_hand(self.deck)
                # The defender takes the cards, the new main attacker is the one
                # to the left of the defender (or the second attacker)
                self.new_trick(self.attackers[1 % len(self.attackers)].name)
        elif action[0] == 'PassAttack':
            # The person passed on attacking, the next attacker may play
            self.current_attacker = (self.current_attacker + 1) % len(self.attackers)
            self.player_to_play = self.attackers[self.current_attacker]

            # Check if we have an entire round of people that do not want to attack
            if self.player_to_play == self.last_played_attacker:
                # The defender defended successfully
                # Let everybody draw cards
                for p in self.draw_order:
                    self.deck = p.fill_hand(self.deck)

                # Perform checks and prints
                assert self.cards_to_defend == []  # Cards still need to be defended
                if self.print_info:
                    print(f'The card pairs [{"".join(f"({str(p[0])[1:]}, {str(p[1])[1:]})" for p in self.pairs_finished)}] are removed from the game')
                # Initialize a new trick with the defender as main attacker
                self.new_trick(self.defender.name)
        elif action[0] == 'Reflect':
            card_played = self.defender.discard_card(self, action[1][0], action[1][1])
            # The new defender sits left of the current defender, the main attacker
            # stays the same and the current cards need to be defended
            self.last_played_attacker = self.player_to_play
            num_old_attackers = len(self.attackers)
            new_defender = self.attackers.pop(1 % num_old_attackers)
            self.attackers.insert(1 % num_old_attackers, self.defender)
            self.defender = new_defender
            self.draw_order = self.attackers + [self.defender]
            # The current attacker
            self.attackers = self.attackers[1:] + [self.attackers[0]]
            self.cards_to_defend.append(card_played)
            self.current_action = 'Defend'
            self.player_to_play = self.defender
        elif action[0] == 'ReflectTrump':
            # By only having to show the trump you can reflect the cards
            suit, value = action[1]
            # You must be the defender to do this
            assert self.player_to_play == self.defender
            # Everyone now knows you have that trump card but you do not lose the card
            self.player_to_play.discard_card(self, suit, value, remove=False)
            # This card loses its ability to reflect for the rest of this trick
            self.reflected_trumps.append((suit, value))
            # The new defender sits left of the current defender,
            # the main attacker stays the same
            # and the current cards need to be defended
            self.last_played_attacker = self.player_to_play
            num_old_attackers = len(self.attackers)
            new_defender = self.attackers.pop(1 % num_old_attackers)
            self.attackers.insert(1 % num_old_attackers, self.defender)
            self.defender = new_defender
            self.draw_order = self.attackers + [self.defender]
            # The current attacker
            self.attackers = self.attackers[1:] + [self.attackers[0]]
            self.current_action = 'Defend'
            self.player_to_play = self.defender
        else:
            raise NotImplementedError('Action to execute not implemented.')


    def make_deepcopy(self):
        """Returns a deepcopy of the GameTree, faster than deepcopy"""
        ### Deepcopy code for checks
        # from copy import deepcopy
        # new = deepcopy(self)
        # new.print_info = False
        # return new
        ### Faster code
        new = GameTree(0, 0, 0, False)
        players = [p.make_copy() for p in self.players]

        ### We copy all the players in all the places
        new.players = []
        player_ids = {}  # dict of all the players with their copy
        for p in self.players:
            copy_p = p.make_copy()
            player_ids[id(p)] = copy_p
            new.players.append(copy_p)

        new.attackers = [player_ids[id(p)] for p in self.attackers]
        new.draw_order = [player_ids[id(p)] for p in self.draw_order]
        new.player_to_play = player_ids[id(self.player_to_play)]
        new.last_played_attacker = player_ids.get(id(self.last_played_attacker), None)
        new.defender = player_ids.get(id(self.defender), None)
        new.loser = player_ids.get(id(self.loser), None)

        ### Copy different, more general information
        new.is_end_state = self.is_end_state
        new.computer_shuffle = self.computer_shuffle
        new.all_cards = self.all_cards
        new.current_action = self.current_action
        new.current_attacker = self.current_attacker
        new.attacker_to_start_throwing = self.attacker_to_start_throwing
        new.reflected_trumps = self.reflected_trumps.copy()  # (suit, value) pairs
        new.history = self.history.copy()
        new.print_info = False

        ### And now we copy all the cards changing each card in all places
        new.card_collection = []
        card_ids = {}  # dict of all the cards with their copy
        for card in self.card_collection:
            # Change card on all different places
            copy_card = card.make_copy()
            card_ids[id(card)] = copy_card
            new.card_collection.append(copy_card)

        for player_idx, p in enumerate(self.players):
            new.players[player_idx].hand = [card_ids[id(c)] for c in p.hand]
        new.deck = [card_ids[id(c)] for c in self.deck]
        new.cards_to_defend = [card_ids[id(c)] for c in self.cards_to_defend]
        new.pairs_finished = [(card_ids[id(p[0])], card_ids[id(p[1])]) for p in self.pairs_finished]
        return new


    def next(self):
        """Chooses and performs an action/move"""
        # Check if this node is terminal
        if self.is_end_state:
            raise BaseException('This was an end state')

        # Choose an action
        action = self.player_to_play.choose_action(self)
        # Display the action
        if self.print_info or not self.computer_shuffle:
            if action[0] in ['Attack', 'Defend', 'Reflect']:
                print(f'Action {action[0]} with card {"♣♠♥♦"[action[1][0]] + "6789*JQKA"[action[1][1]]} was chosen by {self.player_to_play}')
            elif action[0] == 'ThrowCards':
                print(f'Action {action[0]} with {action[1]} was chosen by {self.player_to_play}')
            else:
                print(f'Action {action} was chosen by {self.player_to_play}')

        # Execute the action
        self.execute_action(action)


if __name__ == '__main__':
    import random
    random.seed(2)
    # Note the main attacker should be specified

    # The players can be one of ISMCTS, ISMCTSFPV, DeterminizedMCTS, Random, Human
    players = [DeterminizedMCTS('Player1', deals=5, rollouts=200, expl_const=.8),
               Random('OTHER')]

    # If the computer must shuffle the deck of cards instead the player in real-life
    # computer_shuffle = False
    computer_shuffle = True

    game = GameTree(players, computer_shuffle, main_attacker='Player1')
    while not game.is_end_state:
        game.next()

    print()
    print(f'Game is lost by {game.loser}')
    # print([str(card) for card in game.card_collection])
