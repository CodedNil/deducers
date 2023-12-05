extends ColorRect

var leaderboard_item_scene = preload("res://Leaderboard/LeaderboardItem.tscn")

var player_scores = {"PlyrA": 3, "PlyrB": 1}


func _ready():
	var items_container = $MarginContainer/VBoxContainer

	# Add new items if needed
	while items_container.get_child_count() < len(player_scores) + 1:
		var new_item = leaderboard_item_scene.instantiate()
		items_container.add_child(new_item)

	# Remove excess items if needed
	while items_container.get_child_count() > len(player_scores) + 1:
		var last_item = items_container.get_child(items_container.get_child_count() - 1)
		items_container.remove_child(last_item)
		last_item.queue_free()

	# Update leaderboard items with player scores
	var index = 1
	for player_name in player_scores:
		var item = items_container.get_child(index) as Control
		item.get_node("ColorName/PlayerName").set_text(player_name)
		item.get_node("ColorScore/PlayerScore").set_text(str(player_scores[player_name]))
		index += 1
