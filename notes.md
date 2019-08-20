# Multiplayer Structure

## Client

- Transform hardware input into server commands
- Receive state updates from server
- Render state to screen
- Handle client side configuration
- Wrap a simulator for local play

## Server

- Owns a simulator to simulate game rules
- Send state updates to connected clients
- Accept commands from clients
- Handles pre-game lobbies

## Simulator

- Simulate game rules
- Can be executed on local or remote machine
- Creates state delta

## TODO

- Put common code into module (eg, `State` and game simulation)
- Handle multiple connections to the same game state (ei, two players)
  - At the moment, it's one game per client connection, where that client plays both players.

1. Create game server when a client connects.
2. Wait for an additional client to connect.
3. Accept commands from either player connection and broadcast game state changes to them.
4. Any excess client connections become spectators that get broadcasted game state, but cannot send commands.
