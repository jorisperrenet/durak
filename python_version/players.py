from abc import ABC, abstractmethod
from itertools import permutations
import random

from tools import Card
from tools import choose_random, choose_random_action
from mcts import MCTreeFPV, MCTree, ISMCTree


class Player(ABC):
    def __init__(self, name):
        self.name = name
        self.hand = []

    def __str__(self):
        return f'Player({self.name})'

    def fill_hand(self, deck):
        """Fills the hand up to 6 cards drawing from the deck"""
        # Calculate the number of cards to draw
        to_draw = 6 - len(self.hand)
        if to_draw <= 0:
            return deck
        # Draw the cards
        for _ in range(to_draw):
            # Check if the deck is empty
            if len(deck) == 0:
                # If so, do not draw any cards
                return deck
            # Otherwise draw from the top of the deck
            card_drawn = deck.pop(0)
            # The card is private but unknown to all people
            # even to the person him/herself (we ask the value)
            # later, thus we do not change private/public.
            self.hand.append(card_drawn)

        return deck

    def make_cards_known(self, game_state):
        """Makes the cards in the players hand not unknown"""
        for card in self.hand:
            # i.e. all unknown cards must be known
            if card.is_unknown:
                # check the options for the cards
                unknown = game_state.get_unknown_cards()
                if game_state.computer_shuffle:
                    card.from_suit_value(*choose_random(unknown))
                else:
                    print(f'{self} has drawn card')
                    card.from_input(unknown)
                card.is_private = True

    def possible_card_plays(self, non_public_cards):
        """Returns the (suit, value) pairs this person can play from his hand"""
        poss_plays = set()
        for card in self.hand:
            if card.is_unknown:
                # This card cannot be public and cannot be in the current hand,
                # however, since the private hand will be added, these cards
                # are not dealt with separately.
                poss_plays.update(non_public_cards)
            else:
                # This card is either public or private and known to this person
                poss_plays.add((card.suit, card.value))
        return poss_plays

    def discard_card(self, game_state, suit, value, remove=True):
        """Discards the card from the hand with suit and value"""
        for idx, card in enumerate(self.hand):
            if card.is_unknown:
                # Check if (suit, value) pair in unknown cards (to this player)
                # The unknown cards to this player are all non-public cards minus cards
                # in the (known) hand of this player
                if (suit, value) in game_state.get_non_public_cards():
                    if (suit, value) not in [(c.suit, c.value) for c in self.hand if not c.is_unknown]:
                        # Define the unknown card to be this card
                        card.suit = suit
                        card.value = value
                        break
            else:
                if card.suit == suit and card.value == value:
                    break
        else:
            raise BaseException('Card not possible to discard')
        if remove:
            card_played = self.hand.pop(idx)
        else:
            card_played = self.hand[idx]
        card_played.is_unknown = False
        card_played.is_private = False
        card_played.is_public = True
        return card_played

    def can_throw(self, fallback_identities, cards):
        """Checks if this player can throw away cards"""
        # fallback identities are the options of the cards if it is unknown
        poss = []
        cards_set = set(cards)
        fallback = 0
        for card in self.hand:
            if card.is_unknown:
                fallback += 1
            else:
                identity = (card.suit, card.value)

                # Check if this card has an identity that match a card in cards
                if identity in cards_set:
                    poss.append({identity})
        # Easy case, poss is not big enough to consist of len(cards) cards
        # if len(poss) < len(cards):
        if len(poss) + fallback < len(cards):
            return False
        # Add fallbacks, the minimum amount needed
        poss += [fallback_identities.copy() for _ in range(min(fallback, len(cards)))]
        ### We need to check if we can play cards, having poss
        # # Easy case, one of the cards is not in poss
        # p = set()
        # for i in poss:
        #     p = p.union(i)
        # for c in cards:
        #     if c not in p:
        #         return False
        # Greedy approach: take the nth card from the first allowed poss
        poss2 = [i.copy() for i in poss]
        for c in cards:
            for idx, p in enumerate(poss2):
                if c in p:
                    poss2.pop(idx)
                    break
            else:
                return False
        return True

        # # Otherwise iterating through all options
        # # -> takes a long time
        # for _ in range(len(poss) - len(cards)):
        #     cards.append(0)
        # for perm in permutations(cards, r=len(cards)):
        #     # print(perm, poss)
        #     for idx, card in enumerate(perm):
        #         if card != 0 and card not in poss[idx]:
        #             break
        #     else:
        #         return True
        # return False

    def determinize_hand(self, game_state):
        """Returns a possible determinization for the hand of this player"""
        # First, we make every card in our hand public
        unknown_cards = []
        for card in self.hand:
            if card.is_unknown:
                unknown_cards.append(card)
            else:
                card.is_private = False
                card.is_public = True
        # Check if we need to do anything
        if len(unknown_cards) > 0:
            # We pop from all the unknown cards as possible cards
            unknown = list(game_state.get_non_public_cards())
            for unknown_card in unknown_cards:
                suit, value = unknown.pop(random.randint(0, len(unknown)-1))
                unknown_card.from_suit_value(suit, value)
                unknown_card.is_public = True

    @abstractmethod
    def choose_action(self, game_state):
        """Choose an action (Attack, Defend, Reflect, ...)"""

    @abstractmethod
    def make_copy(self):
        """Returns a copy of self (without copying the cards in the hand)"""

class Random(Player):
    def make_copy(self):
        new = Random(self.name)
        new.hand = self.hand.copy()
        return new

    def choose_action(self, game_state):
        # Choose a random action from all allowed actions
        # The hand of the current player must be known
        # to determine which plays are allowed.
        self.make_cards_known(game_state)

        # Now all cards are known, check the allowed plays
        allowed_actions = game_state.allowed_plays()

        return choose_random_action(allowed_actions)


class Human(Player):
    def make_copy(self):
        new = Human(self.name)
        new.hand = self.hand.copy()
        return new

    def choose_action(self, game_state):
        if game_state.computer_shuffle:
            # The computer shuffles, so we can choose what to do
            # from checking the cards in our hand
            self.make_cards_known(game_state)

            # Now all cards are known, check the allowed plays
            allowed_actions = game_state.allowed_plays()
        else:
            # We must choose an allowed action, however since humans (i.e. real life
            # players) do not need to know their cards (if they shuffle themselves)
            # the allowed plays must return all possible actions with all possible
            # cards this player can have (in that case).
            allowed_actions = game_state.allowed_plays()

        ### Choose action from allowed actions
        action_types = {action[0] for action in allowed_actions}
        print()
        print([str(c) for c in self.hand])
        print([(i[0], i[1]) for i in allowed_actions])
        print(f'What does {self} do?')
        if len(action_types) > 1:
            action_types = sorted(action_types)
            idx = eval(input(f'Choose action from {action_types}: '))
            action_type = action_types[idx]
        else:
            action_type = list(action_types)[0]

        choices = [i[1] for i in allowed_actions if i[0] == action_type]
        if len(choices) == 1:
            return (action_type, choices[0])

        if action_type in ['Attack', 'Defend', 'Reflect', 'ReflectTrump']:
            while True:
                suit = eval(input(f'Suit of the {action_type} card [♣♠♥♦]: '))
                value = eval(input(f'Value of the {action_type} card [6789*JQKA]: '))
                if (suit, value) in choices:
                    break
                print(f'Not valid, try again, the choices are [{" ".join("♣♠♥♦"[c[0]] + "6789*JQKA"[c[1]] for c in choices)}]')
            return (action_type, (suit, value))
        elif action_type in ['Take', 'PassAttack']:
            return (action_type, None)
        elif action_type == 'ThrowCards':
            idx = eval(input(f'Choose throw from {choices}: '))
            return (action_type, choices[idx])
        else:
            raise NotImplementedError
        raise NotImplementedError


class ISMCTSFPV(Player):
    """
    Some notes, NOTE, TODO

    The determinization technique makes use of sampling states from an information
    set and analyzing the corresponding games of perfect information. [download/Information_Set_Monte_Carlo_Tree_Search.pdf]
    This means that for a game like bridge, regular MCTS is used after a random
    sample is taken from the possible hands of the other players.

    This player has implemented the ISMCTS as followes:
        Each time this player gets a choice, he traverses the tree
            like a regular MCTS
        Each time another player gets to make a choice, he plays randomly
    In essence this algorithm is a MCTS, with the nodes being only game state
    in which the perspective player is to play, and edges the combined moves
    of the perspective player and all others untill the perspective players turn.
    """
    def __init__(self, name, rollouts=1000, expl_const=.7, scoring='Winning percentage'):
        self.name = name
        self.hand = []
        self.tree = MCTreeFPV(scoring)
        self.rollouts = rollouts
        self.expl_const = expl_const

    def make_copy(self):
        new = ISMCTSFPV(self.name)
        new.hand = self.hand.copy()
        return new

    def choose_action(self, game_state):
        # Choose an action from all allowed actions
        # by using Information Set Monte Carlo Tree Search.
        # The hand of the current player must be known
        # to determine which plays are allowed.
        self.make_cards_known(game_state)

        # Check if we can only do one action
        # if not, we call the function again on a later point, but
        # this is such a time save that it's worth it
        allowed = game_state.allowed_plays()
        if len(allowed) == 1:
            return allowed[0]

        # Retrieve the search tree from previous iterations
        search_tree = game_state.player_to_play.tree
        # From this point on we do not want to change the original game state
        copied_state = game_state.make_deepcopy()
        # NOTE: Doing this also changes this class to a new player class.
        #       ->  Be careful! Only compare names from this point forward.
        # We must view everything from the perspective of this player
        # thus all the private cards in others people hands are reset
        # since they are unknown to us.
        for card in copied_state.card_collection:
            if card.is_private and card not in self.hand:
                # Reset value (keeping the same memory address)
                card.reset()

        # Do rollouts
        search_tree.do_rollouts(copied_state, self.rollouts, self.expl_const)
        # Choose action
        action_to_play, score = search_tree.choose_action(copied_state)

        if game_state.print_info:
            if search_tree.scoring == 'Visit count':
                print(f'{self} expects to not lose with {score} visits')
            else:
                print(f'{self} expects to not lose with {score*100:.2f}%')

        # Reuse the (now empty) search tree for when information needs
        # to be self over.
        self.tree = search_tree

        return action_to_play


class DeterminizedMCTS(Player):
    """
    Some notes, NOTE, TODO

    The determinization technique makes use of sampling states from an information
    set and analyzing the corresponding games of perfect information. [download/Information_Set_Monte_Carlo_Tree_Search.pdf]
    This means that for a game like bridge, regular MCTS is used after a random
    sample is taken from the possible hands of the other players.
    """
    def __init__(self, name, deals=10, rollouts=100, expl_const=.7, scoring='Winning percentage'):
        self.name = name
        self.hand = []
        self.tree = MCTree()
        self.scoring = scoring
        self.rollouts = rollouts  # number of rollouts per deal
        self.deals = deals
        self.expl_const = expl_const

    def make_copy(self):
        new = DeterminizedMCTS(self.name)
        new.hand = self.hand.copy()
        return new

    def random_deal(self, game_state):
        """We make a random deal on the card collection as determinization"""
        # First, we make every card in our hand public
        for card in game_state.player_to_play.hand:
            card.is_private = False
            card.is_public = True
        # We shuffle all the unknown cards (including private cards in other hands)
        unknown = list(game_state.get_non_public_cards())
        random.shuffle(unknown)
        # We define the unknown cards in the card_collection as a random card
        for card in game_state.card_collection:
            if card.is_unknown:
                # We set this card to suit and value
                suit, value = unknown.pop(0)
                card.from_suit_value(suit, value)
                card.is_public = True
            elif card.is_private:
                # We set this card to suit and value
                suit, value = unknown.pop(0)
                card.is_private = False
                card.is_unknown = True  # From our perspective the card in unknown
                card.from_suit_value(suit, value)
                card.is_public = True
        # Now we have all different, all public cards
        return game_state

    def choose_action(self, game_state):
        # Choose an action from all allowed actions
        # by using a determinized Monte Carlo Tree Search.
        # The hand of the current player must be known
        # to determine which plays are allowed.
        self.make_cards_known(game_state)

        # Check if we can only do one action
        # if not, we call the function again on a later point, but
        # this is such a time save that it's worth it
        allowed = game_state.allowed_plays()
        if len(allowed) == 1:
            return allowed[0]

        # We must view everything from the perspective of this player
        # thus all the private cards in others people hands are reset
        # since they are unknown to us.
        copied_state = game_state.make_deepcopy()
        for card in copied_state.card_collection:
            if card.is_private and card not in self.hand:
                # Reset value (keeping the same memory address)
                card.reset()

        total_ratings = {}
        for deal in range(self.deals):
            # From this point on we do not want to change the original game state
            # NOTE: Doing this also changes this class to a new player class.
            #       ->  Be careful! Only compare names from this point forward.
            copied = copied_state.make_deepcopy()
            # Determinize the state
            copied = self.random_deal(copied)
            # Retrieve the search tree from previous iterations
            search_tree = game_state.player_to_play.tree
            # Do rollouts
            action_ratings = search_tree.do_rollouts(copied, self.rollouts, self.expl_const)
            for action, (W, N) in action_ratings.items():
                if action not in total_ratings:
                    total_ratings[action] = [W, N]
                else:
                    total_ratings[action][0] += W
                    total_ratings[action][1] += N

        # We choose the action to perform based on the total statistics
        if self.scoring == 'Visit count':
            action_to_play, (W, N) = max(total_ratings.items(), key=lambda x: x[1][1])
            if game_state.print_info:
                print(f'{self} expects to not lose with {N} visits')
        elif self.scoring == 'Winning percentage':
            action_to_play, (W, N) = max(total_ratings.items(), key=lambda x: x[1][0]/x[1][1])
            if game_state.print_info:
                print(f'{self} expects to not lose with {W/N*100:.2f}%')
        else:
            raise BaseException('Scoring type not known.')

        # Reuse the (now empty) search tree for when information needs
        # to be passed over.
        self.tree = search_tree

        return action_to_play


class ISMCTS(Player):
    """
    For each node we find all children
    afterwards we deal a random hand (determinize) and from it play the
    unexplored / best action, afterwards we remove the determinization for the next player.
    """
    def __init__(self, name, rollouts=100, expl_const=.7, scoring='Winning percentage'):
        self.name = name
        self.hand = []
        self.tree = ISMCTree()
        self.scoring = scoring
        self.rollouts = rollouts
        self.expl_const = expl_const

    def make_copy(self):
        new = ISMCTS(self.name)
        new.hand = self.hand.copy()
        return new

    def choose_action(self, game_state):
        # Choose an action from all allowed actions
        # by using a Information Set Monte Carlo Tree Search.
        # The hand of the current player must be known
        # to determine which plays are allowed.
        self.make_cards_known(game_state)

        # Check if we can only do one action
        # if not, we call the function again on a later point, but
        # this is such a time save that it's worth it
        allowed = game_state.allowed_plays()
        if len(allowed) == 1:
            return allowed[0]

        # From this point on we do not want to change the original game state
        # NOTE: Doing this also changes this class to a new player class.
        #       ->  Be careful! Only compare names from this point forward.
        copied = game_state.make_deepcopy()
        # We must view everything from the perspective of this player
        # thus all the private cards in others people hands are reset
        # since they are unknown to us.
        for card in copied.card_collection:
            if card.is_private and card not in self.hand:
                # Reset value (keeping the same memory address)
                card.reset()
        # Retrieve the search tree from previous iterations
        search_tree = game_state.player_to_play.tree
        # Do rollouts
        action_ratings = search_tree.do_rollouts(copied, self.rollouts, self.expl_const)

        # We choose the action to perform based on the total statistics
        if self.scoring == 'Visit count':
            action_to_play, (W, N) = max(action_ratings.items(), key=lambda x: x[1][1])
            if game_state.print_info:
                print(f'{self} expects to not lose with {N} visits')
        elif self.scoring == 'Winning percentage':
            action_to_play, (W, N) = max(action_ratings.items(), key=lambda x: x[1][0]/x[1][1])
            if game_state.print_info:
                print(f'{self} expects to not lose with {W/N*100:.2f}%')
        else:
            raise BaseException('Scoring type not known.')

        # Reuse the (now empty) search tree for when information needs
        # to be passed over.
        self.tree = search_tree

        return action_to_play
