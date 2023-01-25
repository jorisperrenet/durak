from math import log, sqrt

from tools import choose_random_action


class MCNodeEnd:
    """Game states reached from playing an action as the perspective player"""
    W = 0  # Score
    N = 0  # Number of traversals
    is_explored = False

    def get_N_W(self):
        return (self.N, self.W)

class MCNodeChoose:
    """Game states from which the perspective players can choose an action"""
    W = 0  # Score
    N = 0  # Number of traversals

    def __init__(self, game_state):
        self.game_state = game_state
        self.is_end_state = game_state.is_end_state
        self.is_explored = False
        # The children are of the form {
        #   last_action_played: MCNodeEnd
        # }
        self.children = {}

    def uct_select(self, expl_const):
        # There are some children as this node is explored and non-terminal
        kids = []
        for action_played, mcnodeend in self.children.items():
            N, W = mcnodeend.get_N_W()
            if N > 0:  # If the child is traversed
                kids.append((action_played, N, W))

        assert self.N > 0  # Check if node is traversed
        assert len(kids) > 0  # At least one of the children must be traversed

        const = expl_const * sqrt(log(self.N))
        def uct(args):
            _, N, W = args
            return W / N + const * N**(-.5)

        # Choose best kid
        best = max(kids, key=uct)
        return (best[0], self.children[best[0]])


class MCTreeFPV:
    """Can perform a ISMCTS from the game_state onwards (first person view)"""
    def __init__(self, scoring):
        # Keep track of all traversals from the room game state
        self.current_traversals = {}  # {game_state.get_id(): MCNodeChoose}
        self.mctsnode = None
        self.expl_const = .7
        self.player_name = None
        self.scoring = scoring

    def choose_action(self, game_state):
        game_id = game_state.get_id()
        # Check if the game state is known
        assert game_id in self.current_traversals
        mctsnode = self.current_traversals[game_id]

        # Choose the action with the highest score
        kids = []
        best, action_to_play = -1, None
        for action, node in mctsnode.children.items():
            if node.N > 0:
                if self.scoring == 'Winning percentage':
                    score = node.W / node.N  # winning percentage
                elif self.scoring == 'Visit count':
                    score = node.N  # visit count
                else:
                    raise ValueError('Scoring type not known')
                # print(action, node.W, node.N)
                if score > best:
                    best = score
                    action_to_play = action
        # Make sure the action is found
        assert action_to_play is not None
        # Clear memory
        self.current_traversals.clear()
        return action_to_play, best

    def do_rollouts(self, game_state, rollouts=1000, expl_const=.7):
        """We traverse this tree where each node is the state from which
        the player (the perspective player) can choose an action.
        """
        self.expl_const = expl_const
        # Also, because of inperfect information, the search tree cannot be reused
        # Create a new node
        self.mctsnode = MCNodeChoose(game_state)
        self.current_traversals[game_state.get_id()] = self.mctsnode

        # Set the name of the perspective player
        if self.player_name == None:
            # The name is a string and as strings are immutable this will
            # not change over time
            self.player_name = game_state.player_to_play.name
        # Only calls from the perspective player can be made in this tree
        # otherwise the Choose and End nodes will not be configured correctly.
        # Although this would then still return the correct answer, (making a
        # new node) this is a simple check to observe what happens.
        assert self.player_name == game_state.player_to_play.name

        # Perform the rollouts/traversals
        for rollout in range(rollouts):
            print(f'Doing rollout {rollout} for {self.player_name}', end='\r')
            # Do the (IS)MCTS
            path, leaf_node = self.select()
            self.expand(leaf_node)
            name_of_loser = self.simulate(leaf_node)
            self.backpropagate(path, name_of_loser)
        print(' '*50, end='\r')

    def select(self):
        """Select a leaf node from the tree"""
        # Traverse the tree as long as possible
        mctsnode = self.mctsnode
        # Remember the traversed path
        path = []
        while True:
            path.append(mctsnode)
            # Check if we stop traversing
            if not mctsnode.is_explored:
                # This game never got expanded, end
                return path, mctsnode

            if mctsnode.is_end_state:
                # An end state, thus this is already a leaf
                return path, mctsnode

            # Otherwise, we go to a unexplored child
            for action, node in mctsnode.children.items():
                if not node.is_explored:
                    node.is_explored = True
                    break
            else:
                # If all children are explored, we select a child according
                # to the UCT scoring.
                action, node = mctsnode.uct_select(self.expl_const)
            # node is a MCNodeEnd, of which the N and W must also be increased
            # for calculating the UCT correctly
            path.append(node)

            # Do random plays until player_to_play == self.player_name
            # i.e. the perspective player may play again
            game = mctsnode.game_state.make_deepcopy()
            # Perform the action of the perspective player
            game.execute_action(action)
            # Perform random allowed actions
            while not game.is_end_state and game.player_to_play.name != self.player_name:
                allowed = game.allowed_plays()
                action = choose_random_action(allowed)
                game.execute_action(action)

            # Check if this game state is known
            game_id = game.get_id()
            if game_id in self.current_traversals:
                # Continue traversing this node
                mctsnode = self.current_traversals[game_id]
            else:
                # Not a known game state, make new node and return it
                leaf_node = MCNodeChoose(game)
                path.append(leaf_node)
                return path, leaf_node

    def expand(self, leaf_node):
        """Expand the leaf node from the tree, i.e. simulate all possible actions"""
        if leaf_node.is_end_state:
            # Nothing to do
            return None

        # Simulate all possible actions
        game = leaf_node.game_state
        self.current_traversals[game.get_id()] = leaf_node
        # Check which actions we're allowed to do
        allowed = game.allowed_plays()
        # Initialize the children of the leaf node
        leaf_node.children = {action: MCNodeEnd() for action in allowed}
        leaf_node.is_explored = True

    def simulate(self, leaf_node):
        """Play random game until an end state is reached, return loser"""
        if leaf_node.is_end_state:
            return leaf_node.game_state.loser.name

        # Choose a random action from the children of the leaf node
        action = choose_random_action(list(leaf_node.children.keys()))
        # Execute the action
        game = leaf_node.game_state.make_deepcopy()
        game.execute_action(action)

        # Traverse the tree randomly
        while not game.is_end_state:
            # Execute a random allowed action
            allowed = game.allowed_plays()
            action = choose_random_action(allowed)
            game.execute_action(action)

        # Return the loser of the game (the name of since strings are immutable)
        return game.loser.name

    def backpropagate(self, path, name_of_loser):
        """Increases visit counts in the tree (and winning count, etc)"""
        for mctsnode in reversed(path):  # bottom up traversal
            # Increase visit count
            mctsnode.N += 1

            # Increase reward
            if name_of_loser != self.player_name:
                # This person did not lose
                mctsnode.W += 1


class MCNode:
    """Game states from which any player can choose an action"""
    W = 0  # Score
    N = 0  # Number of traversals

    def __init__(self, parent=None, game_state=None):
        if game_state is None:
            self.game_state = None
            self.is_end_state = None
        else:
            self.game_state = game_state
            self.is_end_state = game_state.is_end_state
        self.is_explored = False
        self.parent = parent  # MCNode
        # The children are of the form {
        #   last_action_played: MCNode
        # }
        self.children = {}

    def get_game_state(self):
        """For preventing from finding all unsearched game states"""
        if self.game_state is not None:
            return self.game_state
        # Execute the action contained in the edge above this node
        # i.e. in the children of parent
        assert self.parent is not None
        for action_to_perform, child in self.parent.children.items():
            if id(child) == id(self):
                game = self.parent.get_game_state().make_deepcopy()
                game.execute_action(action_to_perform)
                self.is_end_state = game.is_end_state
                return game
        else:
            raise BaseException('Unable to find child in the children of its parent')

    def uct_select(self, expl_const):
        # There are some children as this node is explored and non-terminal
        kids = []
        for action_played, mctsnode in self.children.items():
            N, W = mctsnode.N, mctsnode.W
            if N > 0:  # If the child is traversed
                kids.append((action_played, N, W))

        assert self.N > 0  # Check if node is traversed
        assert len(kids) > 0  # At least one of the children must be traversed

        const = expl_const * sqrt(log(self.N))
        def uct(args):
            _, N, W = args
            return W / N + const * N**(-.5)

        # Choose best kid
        best = max(kids, key=uct)
        return (best[0], self.children[best[0]])


class MCTree:
    """Can perform a MCTS from the game_state onwards"""
    def __init__(self):
        # Keep track of all traversals from the room game state
        self.mctsnode = None
        self.expl_const = .7
        self.player_name = None


    def do_rollouts(self, game_state, rollouts=1000, expl_const=.7):
        """We traverse this tree where each node is the state from which
        the player (the perspective player) can choose an action.
        """
        self.expl_const = expl_const
        # Also, because of imperfect information, the search tree cannot be reused
        # Create a new node
        self.mctsnode = MCNode(game_state=game_state)

        # Perform the rollouts/traversals
        for rollout in range(rollouts):
            print(f'Doing rollout {rollout} for {game_state.player_to_play.name}', end='\r')
            # Do the (IS)MCTS
            leaf_node = self.select()
            self.expand(leaf_node)
            name_of_loser = self.simulate(leaf_node)
            self.backpropagate(leaf_node, name_of_loser)
        print(' '*50, end='\r')

        # Return the information of each action with their performance
        dct = {}
        for action_played, child in self.mctsnode.children.items():
            dct[action_played] = (child.W, child.N)
        # Clear cache
        self.mctsnode = None
        return dct

    def select(self):
        """Select a leaf node from the tree"""
        # Traverse the tree as long as possible
        mctsnode = self.mctsnode
        # Remember the traversed path
        while True:
            # Check if we stop traversing
            if not mctsnode.is_explored:
                # This game never got expanded, end
                return mctsnode

            # If we do not have the corresponding game state yet we do not know
            # if it is an end state
            if mctsnode.is_end_state is None:
                mctsnode.get_game_state()

            if mctsnode.is_end_state:
                # An end state, thus this is already a leaf
                return mctsnode

            # Otherwise, we go to a unexplored child
            for action, node in mctsnode.children.items():
                if not node.is_explored:
                    break
            else:
                # If all children are explored, we select a child according
                # to the UCT scoring.
                action, node = mctsnode.uct_select(self.expl_const)

            # We perform action (but do not have to execute it)
            mctsnode = node

    def expand(self, leaf_node):
        """Expand the leaf node from the tree, i.e. simulate all possible actions"""
        if leaf_node.is_end_state is None:
            leaf_node.get_game_state()

        if leaf_node.is_end_state:
            # Nothing to do
            return None

        # Simulate all possible actions
        game = leaf_node.get_game_state()
        # Check which actions we're allowed to do
        allowed = game.allowed_plays()
        # Initialize the children of the leaf node
        leaf_node.children = {action: MCNode(parent=leaf_node) for action in allowed}
        leaf_node.is_explored = True

    def simulate(self, leaf_node):
        """Play random game until an end state is reached, return loser"""
        assert leaf_node.is_end_state is not None
        if leaf_node.is_end_state:
            return leaf_node.get_game_state().loser.name

        # Choose a random action from the children of the leaf node
        action = choose_random_action(list(leaf_node.children.keys()))
        # Execute the action
        game = leaf_node.get_game_state().make_deepcopy()
        game.execute_action(action)

        # Traverse the tree randomly
        while not game.is_end_state:
            # Execute a random allowed action
            allowed = game.allowed_plays()
            action = choose_random_action(allowed)
            game.execute_action(action)

        # Return the loser of the game (the name of since strings are immutable)
        return game.loser.name

    def backpropagate(self, mctsnode, name_of_loser):
        """Increases visit counts in the tree (and winning count, etc)"""
        while mctsnode is not None:  # bottom up traversal
            # Increase visit count
            mctsnode.N += 1

            # Increase reward
            if mctsnode.parent is not None:
                # The player to play before he played the action (on the edge) is the one
                # that has to choose the action to go to this node or not.
                if mctsnode.parent.get_game_state().player_to_play.name != name_of_loser:
                    # This person did not lose
                    mctsnode.W += 1
            mctsnode = mctsnode.parent


class ISMCNode(MCNode):
    """Game states from which the any player can choose an action"""
    def uct_select(self, allowed_plays, expl_const):
        # There are some children as this node is explored and non-terminal
        kids = []
        for action_played, mctsnode in self.children.items():
            if action_played in allowed_plays:
                N, W = mctsnode.N, mctsnode.W
                if N > 0:  # If the child is traversed
                    kids.append((action_played, N, W))

        assert self.N > 0  # Check if node is traversed
        assert len(kids) > 0  # At least one of the children must be traversed

        const = expl_const * sqrt(log(self.N))
        def uct(args):
            _, N, W = args
            return W / N + const * N**(-.5)

        # Choose best kid
        best = max(kids, key=uct)
        return (best[0], self.children[best[0]])


class ISMCTree:
    """Can perform an ISMCTS from the game_state onwards"""
    def __init__(self):
        # Keep track of all traversals from the room game state
        self.mctsnode = None
        self.expl_const = .7
        self.player_name = None


    def do_rollouts(self, game_state, rollouts=1000, expl_const=.7):
        """We traverse this tree where each node is the state from which
        the player (the perspective player) can choose an action.
        """
        self.expl_const = expl_const
        # TODO: This search tree can be reused
        # Create a new node
        self.mctsnode = ISMCNode(game_state=game_state)

        # Perform the rollouts/traversals
        for rollout in range(rollouts):
            print(f'Doing rollout {rollout} for {game_state.player_to_play.name}', end='\r')
            # Do the (IS)MCTS
            leaf_node = self.select()
            self.expand(leaf_node)
            name_of_loser = self.simulate(leaf_node)
            self.backpropagate(leaf_node, name_of_loser)
        print(' '*50, end='\r')

        # Return the information of each action with their performance
        dct = {}
        for action_played, child in self.mctsnode.children.items():
            dct[action_played] = (child.W, child.N)
        # Clear cache
        self.mctsnode = None
        return dct

    def select(self):
        """Select a leaf node from the tree"""
        # Traverse the tree as long as possible
        mctsnode = self.mctsnode
        # Remember the traversed path
        while True:
            # Check if we stop traversing
            if not mctsnode.is_explored:
                # This game never got expanded, end
                return mctsnode

            # If we do not have the corresponding game state yet we do not know
            # if it is an end state
            if mctsnode.is_end_state is None:
                mctsnode.get_game_state()

            if mctsnode.is_end_state:
                # An end state, thus this is already a leaf
                return mctsnode

            # Otherwise, determinize the game
            copied = mctsnode.get_game_state().make_deepcopy()
            copied.player_to_play.determinize_hand(copied)
            allowed = copied.allowed_plays()
            allowed = [(i[0], i[1]) for i in allowed]  # stripped weights
            # Choose an unexplored allowed play
            for action in allowed:
                # Unforseen child (ThrowCards propably)
                if action not in mctsnode.children:
                    mctsnode.children[action] = ISMCNode(parent=mctsnode)
                # Check if the child is unexplored
                if not mctsnode.children[action].is_explored:
                    node = mctsnode.children[action]
                    break
            else:
                # If all children are explored, we select a child according
                # to the UCT scoring.
                action, node = mctsnode.uct_select(allowed, self.expl_const)

            # We perform action (but do not have to execute it)
            mctsnode = node

    def expand(self, leaf_node):
        """Expand the leaf node from the tree, i.e. simulate all possible actions"""
        if leaf_node.is_end_state is None:
            leaf_node.get_game_state()

        if leaf_node.is_end_state:
            # Nothing to do
            return None

        # Simulate all possible actions
        game = leaf_node.get_game_state()
        # Check which actions we're allowed to do
        allowed = game.allowed_plays()
        # Initialize the children of the leaf node
        leaf_node.children = {(action[0], action[1]): ISMCNode(parent=leaf_node) for action in allowed}
        leaf_node.is_explored = True

    def simulate(self, leaf_node):
        """Play random game until an end state is reached, return loser"""
        assert leaf_node.is_end_state is not None
        if leaf_node.is_end_state:
            return leaf_node.get_game_state().loser.name

        # Determinize
        copied = leaf_node.get_game_state().make_deepcopy()
        copied.player_to_play.determinize_hand(copied)
        allowed = copied.allowed_plays()
        # Choose a random allowed action (with this determinization)
        action = choose_random_action(allowed)
        # Execute the action
        game = leaf_node.get_game_state().make_deepcopy()
        game.execute_action(action)

        # Traverse the tree randomly
        while not game.is_end_state:
            # Execute a random allowed action
            allowed = game.allowed_plays()
            action = choose_random_action(allowed)
            game.execute_action(action)

        # Return the loser of the game (the name of since strings are immutable)
        return game.loser.name

    def backpropagate(self, mctsnode, name_of_loser):
        """Increases visit counts in the tree (and winning count, etc)"""
        while mctsnode is not None:  # bottom up traversal
            # Increase visit count
            mctsnode.N += 1

            # Increase reward
            if mctsnode.parent is not None:
                # The player to play before he played the action (on the edge) is the one
                # that has to choose the action to go to this node or not.
                if mctsnode.parent.get_game_state().player_to_play.name != name_of_loser:
                    # This person did not lose
                    mctsnode.W += 1
            mctsnode = mctsnode.parent

