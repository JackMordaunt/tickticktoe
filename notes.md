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
