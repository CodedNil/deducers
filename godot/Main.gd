extends Control

const PORT = 4433


func _ready():
	# Start paused.
	get_tree().paused = true

	# Set random player name
	$ConnectUI/ColorRect/HBoxContainer/PlayerName.text = "Player" + str(randi() % 1000)

	multiplayer.connect("peer_connected", Callable(self, "_player_connected"))
	multiplayer.connect("peer_disconnected", Callable(self, "_player_disconnected"))

	# Automatically start the server in headless mode.
	if DisplayServer.get_name() == "headless":
		print("Automatically starting dedicated server.")
		_on_host_pressed.call_deferred()


func _player_connected(id):
	print("Player connected: ", id)
	# Add to player list, update leaderboard, etc.


func _player_disconnected(id):
	print("Player disconnected: ", id)
	# Remove from player list, update leaderboard, etc.


func _on_host_pressed():
	print("Starting multiplayer server.")
	# Start as server.
	var peer = ENetMultiplayerPeer.new()
	peer.create_server(PORT)
	if peer.get_connection_status() == MultiplayerPeer.CONNECTION_DISCONNECTED:
		OS.alert("Failed to start multiplayer server.")
		return
	multiplayer.multiplayer_peer = peer
	start_game()


func _on_connect_pressed():
	print("Starting multiplayer client.")
	# Start as client.
	var txt: String = $ConnectUI/ColorRect/HBoxContainer/Remote.text
	if txt == "":
		OS.alert("Need a remote to connect to.")
		return
	var peer = ENetMultiplayerPeer.new()
	peer.create_client(txt, PORT)
	if peer.get_connection_status() == MultiplayerPeer.CONNECTION_DISCONNECTED:
		OS.alert("Failed to start multiplayer client.")
		return
	multiplayer.multiplayer_peer = peer
	start_game()


func start_game():
	# Hide the UI and unpause to start the game.
	$ConnectUI.hide()
	get_tree().paused = false
