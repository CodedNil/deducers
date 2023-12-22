use dioxus::prelude::*;

pub fn render(tutorial_open: &UseState<bool>) -> LazyNodes<'_, '_> {
    rsx! {
        div { class: "dialog floating tutorial", top: if *tutorial_open.get() { "50%" } else { "-100%" },
            p {
                "Welcome to the intriguing world of Deducers! Here's how you can become a master deducer in this multiplayer twist on 20 Questions:"
            }
            p {
                strong { "The Game Board:" }
                " At the start, two items will be in play, listed under columns '1' and '2'. The names of these items are a mystery, represented by simple words like 'Bird', 'Mountain', or 'Phone'."
            }
            p {
                strong { "Collect Coins:" }
                " You'll earn coins passively as time goes by. Keep an eye on your coin balance!"
            }
            p {
                strong { "Submit Questions:" }
                " Use your coins to ask questions that will help you deduce the items. Think strategically! For a higher coin cost submit questions masked, other players won't see your question, only the answer."
            }
            p {
                strong { "Question Queue:" }
                " Your submitted questions enter a queue. Every 10 seconds, the question with the most votes is asked. Vote wisely to uncover the clues you need."
            }
            p {
                strong { "Revealing Answers:" }
                " As questions are asked, each item will reveal its answers as 'Yes', 'No', 'Maybe', or 'Unknown'. These clues are vital to your deduction process."
            }
            p {
                strong { "Make Your Guess:" }
                " If you think you've cracked it, spend coins to guess the item. The sooner you guess an item correctly, the more points you get."
            }
            p {
                strong { "New Items:" }
                " After every 5th question, a new item appears, keeping the game fresh and exciting. Keep track of all items and use your questions to reveal their secrets."
            }
            p { "Happy deducing, and may the most astute player win!" }
            button {
                onclick: move |_| {
                    tutorial_open.set(false);
                },
                "OK"
            }
        }
    }
}
