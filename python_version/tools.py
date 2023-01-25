import random


def choose_random(lst, weights=None):
    """Choose a random element"""
    if weights is None:
        return random.choice(list(lst))
    else:
        return random.choices(list(lst))[0]

def choose_random_action(poss_actions):
    """Returns a random action from poss_actions with and without weights"""
    # Check if we used weights
    if len(poss_actions[0]) == 3:
        # Weights are used
        return choose_random(
                list(map(lambda x: (x[0], x[1]), poss_actions)),
                weights=list(map(lambda x: x[2], poss_actions))
            )
    else:
        # Weights are not used
        return choose_random(poss_actions)


class Card:
    """A regular (durak) playing card

    Note that cards can be unknown to all,
    private to the one holding the card
    or known to all.
    """
    suit = None
    value = None
    trump_suit = None
    # There is always one (and only one) of the following true
    is_public = False  # does everyone know the card
    is_private = False  # does the person holding the card know its value
    is_unknown = True  # does no one know the card

    def __str__(self):
        if self.is_unknown:
            return '???'
        else:
            if self.is_private:
                string = 'P'
            else:
                string = 'A'
            assert self.suit is not None  # thought this was not unknown
            string += '♣♠♥♦'[self.suit]
            string += '6789*JQKA'[self.value]
            return string

    def __eq__(self, other):
        return self.suit == other.suit and self.value == other.value

    def reset(self):
        """Reset values or make unknown (useful for keeping same memory address)"""
        # Keeping the same memory address is adamant for this program
        # (not only the copy) as otherwise self.card_collection would
        # not be a collection of the cards anymore.
        self.suit = None
        self.value = None
        self.trump_suit = None
        self.is_public = False
        self.is_private = False
        self.is_unknown = True

    def make_copy(self):
        """Returns a copy of the card"""
        new = Card()
        new.suit = self.suit
        new.value = self.value
        new.trump_suit = self.trump_suit
        new.is_public = self.is_public
        new.is_private = self.is_private
        new.is_unknown = self.is_unknown
        return new

    def is_trump(self):
        """Checks if the current card is a trump card or not"""
        return self.suit == self.trump_suit

    def from_input(self, possible):
        """Get the suit and value of the card from the input

        Args:
            - possible: The (suit, value) allowed options.

        Remark: remember to set private/public permissions
        """

        # We cannot overwrite a current value
        assert self.is_unknown

        while True:
            suit = eval(input('Suit of the card [♣♠♥♦]: '))
            value = eval(input('Value of the card [6789*JQKA]: '))
            if (suit, value) in possible:
                break
            print(f'Not valid, try again (one of {" ".join("♣♠♥♦"[i[0]] + "6789*JQKA"[i[1]] for i in possible)})')
        self.suit = suit
        self.value = value
        self.is_unknown = False

    def from_suit_value(self, suit, value):
        """Set the suit and value of the card

        Remark: remember to set private/public permissions
        """
        # We cannot overwrite a current value
        assert self.is_unknown
        self.suit = suit
        self.value = value
        self.is_unknown = False
