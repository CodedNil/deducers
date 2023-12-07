extends HBoxContainer


func _on_vote_button_pressed():
	var button = $ColorRect3/HBoxContainer/VoteButton
	var button_id = button.get_meta("button_id")
	var root = get_tree().get_root().get_child(0)
	root.call("on_question_queue_vote_pressed", button_id)
