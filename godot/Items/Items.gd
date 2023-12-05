extends ColorRect

var items_header_scene = preload("res://Items/ItemsHeader.tscn")
var items_item_scene = preload("res://Items/ItemsItem.tscn")
var items_answerbox_scene = preload("res://Items/ItemsAnswerBox.tscn")

var questions = {
	0: "Is it a living thing?",
	1: "Is it an object used daily?",
	2: "Is it bigger than a breadbox?",
	3: "Can it be found indoors?",
	4: "Does it have wheels?",
	5: "Is it used for communication?",
	6: "Does it make a sound?",
	7: "Is it man-made?",
	8: "Can it be eaten?",
	9: "Is it a type of vehicle?",
	10: "Is it used for entertainment?",
	11: "Does it require electricity?",
	12: "Is it found in nature?",
	13: "Is it commonly found in a household?",
	14: "Does it change shape or form?"
}

var items = {
	0:
	[
		"Butterfly",
		[
			"0;Y",
			"1;N",
			"2;N",
			"3;M",
			"4;N",
			"5;N",
			"6;Y",
			"7;N",
			"8;Y",
			"9;N",
			"10;N",
			"11;N",
			"12;Y"
		]
	],
	1: ["Crystal", ["5;N", "6;N", "7;M", "8;M", "9;N", "10;N", "11;N", "12;Y"]],  # "0;N", "1;M", "2;M", "3;M", "4;N",
	2: ["Smartphone", ["10;Y", "11;Y", "12;N"]]  # "0;N", "1;Y", "2;N", "3;Y", "4;N", "5;Y", "6;Y", "7;Y", "8;N", "9;N",
}

var answer_colors = {
	"Y": Color(0.25, 0.5, 0.2), "N": Color(0.5, 0.2, 0.2), "M": Color(0.55, 0.35, 0.0)
}


func _ready():
	var header_container = $MarginContainer/ScrollContainer/VBoxContainer/HBoxContainer
	manage_items(header_container, len(items), items_header_scene)

	# Update headers with item text
	var index = 1
	for item_index in items:
		var child = header_container.get_child(index) as Control
		child.get_node("Label").set_text(str(item_index + 1))

		index += 1

	# Get list of questions that are active
	var active_questions = {}
	for question_index in questions:
		for item_index in items:
			var has_answer = false
			for answer_question in items[item_index][1]:
				if answer_question.split(";")[0] == str(question_index):
					has_answer = true
					break

			if has_answer:
				active_questions[question_index] = questions[question_index]
				break

	var items_container = $MarginContainer/ScrollContainer/VBoxContainer
	manage_items(items_container, 20, items_item_scene)

	# Update items with question text
	index = 1
	for question in range(0, 20):
		var num_blanks = 20 - len(active_questions)
		var cur_question = question - num_blanks if question > num_blanks else -1

		var child = items_container.get_child(index) as Control
		child.get_node("ColorRect/MarginContainer/HBoxContainer/Index").set_text(
			str(cur_question) + ": " if question > num_blanks else ""
		)
		child.get_node("ColorRect/MarginContainer/HBoxContainer/Question").set_text(
			active_questions[cur_question] if question > num_blanks else ""
		)

		# Make the right number of answer boxes available
		manage_items(child, len(items), items_answerbox_scene)

		# Colour the answer boxes
		for item_index in items:
			# Get answer if it exists
			var answer = ""
			for answer_question in items[item_index][1]:
				if answer_question.split(";")[0] == str(cur_question):
					answer = answer_question.split(";")[1]
					break

			if answer_colors.has(answer):
				child.get_child(item_index + 1).set_color(answer_colors[answer])
				child.get_child(item_index + 1).get_child(0).set_visible(false)
			else:
				child.get_child(item_index + 1).set_color(Color(0.2, 0.2, 0.2))
				child.get_child(item_index + 1).get_child(0).set_visible(true)

		index += 1


func manage_items(container, count, item_scene):
	# Add new items if needed
	while container.get_child_count() < count + 1:
		var new_item = item_scene.instantiate()
		container.add_child(new_item)

	# Remove excess items if needed
	while container.get_child_count() > count + 1:
		var last_item = container.get_child(container.get_child_count() - 1)
		container.remove_child(last_item)
		last_item.queue_free()
