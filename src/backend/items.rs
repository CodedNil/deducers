use crate::backend::{
    add_chat_message_to_lobby, alert_popup, openai::query_ai, with_lobby, Answer, Item, Lobby, LobbyState, PlayerMessage, Question,
    QueuedQuestionQuizmaster, QuizmasterItem,
};
use anyhow::{anyhow, bail, ensure, Result};
use futures::future::join_all;
use serde::Deserialize;
use std::{cmp::Ordering, collections::HashMap, str::FromStr};

pub fn add_item_to_lobby(lobby: &mut Lobby) {
    if lobby.state != LobbyState::Play || lobby.items_queue.is_empty() {
        return;
    }
    let item_name = lobby.items_queue.remove(0);
    println!("Adding item '{}' to lobby '{}'", item_name, lobby.id);
    lobby.items.push(Item {
        name: item_name,
        id: lobby.items_counter + 1,
        questions: Vec::new(),
    });
    lobby.items_counter += 1;
    for player in lobby.players.values_mut() {
        player.messages.push(PlayerMessage::ItemAdded);
    }
}

#[derive(Deserialize)]
struct AskQuestionResponse {
    answers: Vec<String>,
}

pub async fn ask_top_question(lobby_id: &str) -> Result<()> {
    let (mut question_text, mut question_player, mut question_masked) = (String::new(), String::new(), false);
    let mut question_voters = Vec::new();
    let mut items = Vec::new();
    let mut is_quizmaster = false;

    with_lobby(lobby_id, |lobby| {
        let question = lobby
            .questions_queue
            .iter()
            .max_by_key(|question| question.votes)
            .ok_or_else(|| anyhow!("No questions in queue"))?;

        if question.votes < lobby.settings.question_min_votes {
            bail!("Question needs at least {} votes", lobby.settings.question_min_votes);
        }

        question_text = question.question.clone();
        question_player = question.player.clone();
        question_masked = question.masked;
        question_voters = question.voters.clone();
        items = lobby.items.clone();

        // Remove question from queue
        lobby.questions_queue.retain(|q| q.question != question_text);

        // Reset queue waiting if needed
        if !lobby.questions_queue_active() {
            lobby.questions_queue_countdown = lobby.settings.submit_question_every_x_seconds as f64;
        }

        is_quizmaster = lobby.settings.player_controlled;

        Ok(())
    })?;

    // If quizmaster end here and add to the quizmasters queue
    if is_quizmaster {
        let items_list = items
            .iter()
            .map(|item| QuizmasterItem {
                id: item.id,
                name: item.name.clone(),
                answer: Answer::Maybe,
            })
            .collect();
        with_lobby(lobby_id, |lobby| {
            lobby.quizmaster_queue.push(QueuedQuestionQuizmaster {
                question: question_text.clone(),
                player: question_player.clone(),
                masked: question_masked,
                items: items_list,
                voters: question_voters,
            });
            Ok(())
        })?;
        return Ok(());
    }

    let items_str = items.iter().map(|item| item.name.as_str()).collect::<Vec<&str>>().join(", ");

    // Query with OpenAI API - Get 3 answers for each item to pick the most common answer
    let mut answers_choices: Vec<Vec<Answer>> = Vec::new();
    let mut successful_attempts = 0;
    let mut total_attempts = 0;

    let prompt = format!("u:For each item in this list '{items_str}', in the items usual state answer the question '{question_text}', return compact one line JSON with key answers which is a list of yes, no, maybe or unknown, this is a 20 questions game, British English");
    while successful_attempts < 3 && total_attempts < 3 {
        let mut futures = Vec::new();
        for _ in 0..3 {
            let future = query_ai(&prompt, items.len() * 3 + 20, 1.0, true);
            futures.push(future);
        }

        let responses: Vec<Result<String>> = join_all(futures).await;
        for response in responses.into_iter().flatten() {
            if let Ok(validate_response) = serde_json::from_str::<AskQuestionResponse>(&response) {
                let mut choices = Vec::new();
                for answer_str in validate_response.answers {
                    let answer_str = format!("{}{}", answer_str[..1].to_uppercase(), &answer_str[1..].to_lowercase());
                    if let Ok(answer) = Answer::from_str(&answer_str) {
                        choices.push(answer);
                    }
                }
                if choices.len() == items.len() {
                    answers_choices.push(choices);
                    successful_attempts += 1;
                }
            } else {
                println!("Failed to parse answer response {response}");
            }
        }
        total_attempts += 1;
    }

    // Get most common answer for each item
    let mut answers: Vec<Answer> = Vec::new();
    for item_index in 0..items.len() {
        let mut answer_frequency: HashMap<Answer, usize> = HashMap::new();

        for answers in &answers_choices {
            let answer = answers.get(item_index).expect("Answers should have the same length as items");
            *answer_frequency.entry(*answer).or_insert(0) += 1;
        }

        let most_common_answer = answer_frequency
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map_or(Answer::Unknown, |(ans, _)| ans);

        answers.push(most_common_answer);
    }

    with_lobby(lobby_id, |lobby| {
        if answers.len() != lobby.items.len() {
            bail!("Failed to get answers for question '{question_text}'");
        }

        let question_id = lobby.questions_counter;
        lobby.questions_counter += 1;

        // Ask question against each item
        let mut remove_items = Vec::new();
        for (index, item) in &mut lobby.items.iter_mut().enumerate() {
            let answer = answers.get(index).unwrap_or(&Answer::Unknown);
            item.questions.push(Question {
                player: question_player.clone(),
                id: question_id,
                text: question_text.clone(),
                answer: *answer,
                masked: question_masked,
            });

            // If item has 20 questions, remove the item
            if item.questions.len() >= 20 {
                remove_items.push(item.clone());
                for player_n in lobby.players.values_mut() {
                    player_n.messages.push(PlayerMessage::ItemRemoved(item.id, item.name.clone()));
                }
            }
        }
        if !remove_items.is_empty() {
            for item in &remove_items {
                add_chat_message_to_lobby(
                    lobby,
                    "SYSTEM",
                    &format!("Item {} has been removed from play, it was '{}'", item.id, item.name),
                );
            }
            lobby.items.retain(|i| !remove_items.contains(i));
        }

        if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
            add_item_to_lobby(lobby);
        }

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::QuestionAsked);
        }
        test_game_over(lobby);
        Ok(())
    })
}

pub fn quizmaster_change_answer(lobby_id: &str, player_name: &str, question: &String, item_id: usize, new_answer: Answer) {
    let result = with_lobby(lobby_id, |lobby| {
        ensure!(lobby.state == LobbyState::Play, "Lobby not started");
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow!("Player not found"))?;
        ensure!(player.quizmaster, "Only quizmaster can use this");
        lobby
            .quizmaster_queue
            .iter_mut()
            .find(|q| &q.question == question)
            .and_then(|q| q.items.iter_mut().find(|i| i.id == item_id))
            .map(|i| i.answer = new_answer)
            .ok_or_else(|| anyhow!("Question or item not found"))
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("Change answer failed {error}"));
    }
}

pub fn quizmaster_submit(lobby_id: &str, player_name: &str, question: &str) {
    let result = with_lobby(lobby_id, |lobby| {
        ensure!(lobby.state == LobbyState::Play, "Lobby not started");
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow!("Player not found"))?;
        ensure!(player.quizmaster, "Only quizmaster can use this");

        let question_index = lobby
            .quizmaster_queue
            .iter()
            .position(|q| q.question == question)
            .ok_or_else(|| anyhow!("Question not found"))?;
        let question = lobby.quizmaster_queue.remove(question_index);

        let question_id = lobby.questions_counter;
        lobby.questions_counter += 1;

        let mut remove_items = Vec::new();
        for quizmaster_item in question.items.clone() {
            let item = lobby.items.iter_mut().find(|i| i.id == quizmaster_item.id);
            if let Some(item) = item {
                item.questions.push(Question {
                    player: question.player.clone(),
                    id: question_id,
                    text: question.question.clone(),
                    answer: quizmaster_item.answer,
                    masked: question.masked,
                });

                // If item has 20 questions, remove the item
                if item.questions.len() >= 20 {
                    remove_items.push(item.clone());
                    for player_n in lobby.players.values_mut() {
                        player_n.messages.push(PlayerMessage::ItemRemoved(item.id, item.name.clone()));
                    }
                }
            }
        }

        if !remove_items.is_empty() {
            lobby.items.retain(|i| !remove_items.contains(i));
        }

        if lobby.questions_counter % lobby.settings.add_item_every_x_questions == 0 {
            add_item_to_lobby(lobby);
        }

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::QuestionAsked);
        }

        Ok(())
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("Submission failed {error}"));
    }
}

pub fn quizmaster_reject(lobby_id: &str, player_name: &str, question: &str) {
    let result = with_lobby(lobby_id, |lobby| {
        ensure!(lobby.state == LobbyState::Play, "Lobby not started");
        let player = lobby.players.get(player_name).ok_or_else(|| anyhow!("Player not found"))?;
        ensure!(player.quizmaster, "Only quizmaster can use this");

        let question_index = lobby
            .quizmaster_queue
            .iter()
            .position(|q| q.question == question)
            .ok_or_else(|| anyhow!("Question not found"))?;
        let question = lobby.quizmaster_queue.remove(question_index);

        add_chat_message_to_lobby(
            lobby,
            "SYSTEM",
            &format!("Quizmaster has rejected question '{}'", question.question.clone()),
        );

        // Refund the voters
        for voter in question.voters {
            if let Some(player) = lobby.players.get_mut(&voter) {
                player.coins += 1;
            }
        }
        // Refund the question submitter and send them a message
        if let Some(player) = lobby.players.get_mut(&question.player) {
            player.coins += lobby.settings.submit_question_cost;
            player.messages.push(PlayerMessage::QuestionRejected(question.question));
        }

        Ok(())
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("Rejection failed {error}"));
    }
}

pub fn player_guess_item(lobby_id: &str, player_name: &str, item_choice: usize, guess: &str) {
    let result = with_lobby(lobby_id, |lobby| {
        ensure!(lobby.state == LobbyState::Play, "Lobby not started");
        let player = lobby
            .players
            .get_mut(player_name)
            .ok_or_else(|| anyhow!("Player '{player_name}' not found"))?;
        ensure!(!player.quizmaster, "Quizmaster cannot engage");
        ensure!(player.coins >= lobby.settings.guess_item_cost, "Insufficient coins to guess");

        let item_index = lobby
            .items
            .iter()
            .position(|i| i.id == item_choice)
            .ok_or_else(|| anyhow!("Item not found"))?;
        let item = &lobby.items[item_index];

        player.coins -= lobby.settings.guess_item_cost;
        if item.name.eq_ignore_ascii_case(guess) {
            // Correct guess
            player.score += 20 - item.questions.len();

            for p in lobby.players.values_mut() {
                p.messages
                    .push(PlayerMessage::ItemGuessed(player_name.to_owned(), item.id, item.name.clone()));
            }

            lobby.items.remove(item_index);
            add_chat_message_to_lobby(
                lobby,
                "SYSTEM",
                &format!("'{player_name}' guessed item {item_choice} as '{guess}'",),
            );
            test_game_over(lobby);

            Ok(())
        } else {
            // Incorrect guess
            player.messages.push(PlayerMessage::GuessIncorrect);
            add_chat_message_to_lobby(
                lobby,
                "SYSTEM",
                &format!("'{player_name}' incorrectly guessed '{guess}' for item {item_choice}",),
            );
            bail!("Incorrect guess");
        }
    });
    if let Err(error) = result {
        alert_popup(lobby_id, player_name, &format!("Guess rejected {error}"));
    }
}

pub fn test_game_over(lobby: &mut Lobby) {
    if lobby.items.is_empty() && !lobby.items_queue.is_empty() {
        add_item_to_lobby(lobby);
    }
    if lobby.state == LobbyState::Play && lobby.items.is_empty() {
        lobby.state = LobbyState::Ended;
        lobby.elapsed_time = 0.0;

        // Find winner, player with max score, or if tied multiple players, or if 0 score no winner
        let mut max_score = 0;
        let mut winners = Vec::new();
        for player in lobby.players.values() {
            match player.score.cmp(&max_score) {
                Ordering::Greater => {
                    max_score = player.score;
                    winners.clear();
                    winners.push(player.name.clone());
                }
                Ordering::Equal => {
                    winners.push(player.name.clone());
                }
                Ordering::Less => {}
            }
        }
        if max_score == 0 {
            winners.clear();
        }
        let win_message = if winners.len() > 1 {
            format!("The tied winners are {}!", winners.join(", "))
        } else if winners.is_empty() {
            String::from("The game has ended with no winner!")
        } else {
            format!("The winner is {}!", winners[0])
        };

        for player in lobby.players.values_mut() {
            player.messages.push(PlayerMessage::Winner(win_message.clone()));
        }
    }
}
