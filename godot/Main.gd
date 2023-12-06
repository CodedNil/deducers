extends Control

# Nodes
var http_request: HTTPRequest
@onready var connect_ui = $ConnectUI
@onready var server_ip_line_edit = $ConnectUI/ColorRect/VBoxContainer/ServerIp
@onready var player_name_line_edit = $ConnectUI/ColorRect/VBoxContainer/HBoxContainer/PlayerName
@onready var room_name_line_edit = $ConnectUI/ColorRect/VBoxContainer/HBoxContainer/RoomName

@onready var error_dialog = $ErrorDialog
@onready var error_dialog_label = $ErrorDialog/ColorRect/MarginContainer/VBoxContainer/Label

# Variables to store server and player information
var server_ip: String
var player_name: String
var room_name: String


func _ready():
	http_request = HTTPRequest.new()
	http_request.timeout = 5
	http_request.process_mode = Node.PROCESS_MODE_ALWAYS
	add_child(http_request)
	http_request.request_completed.connect(self._http_request_completed)

	# Start paused
	get_tree().paused = true


func _on_connect_pressed():
	server_ip = server_ip_line_edit.text
	player_name = player_name_line_edit.text
	room_name = room_name_line_edit.text

	# Send connection request
	print("Connecting to server %s, room %s, player %s" % [server_ip, room_name, player_name])
	var url = "http://%s/server/%s/connect/%s" % [server_ip, room_name, player_name]
	var error = http_request.request(url, [], HTTPClient.METHOD_POST)
	if error != OK:
		push_error("An error occurred in the HTTP request.")


func _http_request_completed(result, response_code, _headers, body):
	print(
		(
			"HTTP request completed with result %s, response code %s, body %s"
			% [result, response_code, body]
		)
	)
	if result == HTTPRequest.RESULT_SUCCESS:
		if response_code == 200:
			# Connection successful
			connect_ui.hide()
			get_tree().paused = false
		else:
			# Show error dialog
			error_dialog_label.text = "Connection failed: %s" % [body]
			error_dialog.show()

	else:
		# Show error dialog
		error_dialog_label.text = "Connection failed: %s" % [result]
		error_dialog.show()


func _on_close_error_pressed():
	error_dialog.hide()
