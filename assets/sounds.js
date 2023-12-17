// Check if the audio elements already exist, and if not, create and append them to the document.
function initializeAudioElements() {
    if (!document.getElementById("button-click-sound")) {
        const buttonClickSound = document.createElement("audio");
        buttonClickSound.id = "button-click-sound";
        buttonClickSound.src = "assets/sounds/button_pressed.mp3";
        document.body.appendChild(buttonClickSound);
    }

    if (!document.getElementById("typing-sound")) {
        const typingSound = document.createElement("audio");
        typingSound.id = "typing-sound";
        typingSound.src = "assets/sounds/typing.mp3";
        document.body.appendChild(typingSound);
    }
}

// Function to play the button click sound
function playButtonClickSound() {
    const soundButton = document.getElementById("button-click-sound");
    soundButton.play();
}

// Function to play the typing sound
function playTypingSound() {
    const soundTyping = document.getElementById("typing-sound");
    soundTyping.currentTime = 0; // Reset playback position to the beginning
    soundTyping.play();
}

// Initialize the audio elements
initializeAudioElements();

// Add event listeners for buttons and input fields as shown in previous responses
document.addEventListener("click", function (event) {
    if (event.target && event.target.nodeName === "BUTTON") {
        playButtonClickSound();
    }
});

document.addEventListener("input", function (event) {
    if (event.target && event.target.nodeName === "INPUT") {
        playTypingSound();
    }
});
