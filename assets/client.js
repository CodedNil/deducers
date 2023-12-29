// Check if the audio elements already exist, and if not, create and append them to the document.
function initializeAudio(sound) {
    if (!document.getElementById(sound + "-sound")) {
        const audio = document.createElement("audio");
        audio.id = sound + "-sound";
        audio.src = `assets/sounds/${sound}.mp3`;
        document.body.appendChild(audio);
    }
}

function playSound(sound) {
    // Initialise audio if it doesnt exist
    initializeAudio(sound);
    // Play audio
    const audio = document.getElementById(sound + "-sound");
    audio.currentTime = 0;
    audio.play();
}

// Add event listeners for buttons and input fields as shown in previous responses
document.addEventListener("click", function (event) {
    if (event.target && event.target.nodeName === "BUTTON") {
        playSound("button_pressed");
    }
});

// Clear input fields on form submission
document.addEventListener(
    "submit",
    (event) => {
        if (event.target.tagName === "FORM") {
            setTimeout(() => {
                event.target.querySelectorAll('[data-clear-on-submit="true"]').forEach((element) => {
                    if (element.tagName === "INPUT") {
                        element.value = "";
                    }
                });
            }, 50);
        }
    },
    true
);

document.addEventListener("input", function (event) {
    if (event.target && event.target.nodeName === "INPUT") {
        playSound("typing");
    }

    // Restrict all input fields to their patterns
    if (event.target.tagName === "INPUT" && event.target.pattern) {
        var pattern = new RegExp("^" + event.target.pattern + "$");
        var currentInput = event.target.value;

        // Check the last character entered against the pattern
        if (!pattern.test(currentInput)) {
            // Remove the last character if it doesn't match the pattern
            event.target.value = currentInput.slice(0, -1);
        }
    }
});

// Continuously check and play sounds
const playedSounds = new Set(); // Store played sounds

// Function to play a sound if it hasn't been played before
function playUniqueSound(expirySound) {
    const parts = expirySound.split(";");
    if (parts.length === 2 && parts[1]) {
        const sound = parts[1];
        if (!playedSounds.has(expirySound)) {
            playSound(sound);
            playedSounds.add(expirySound);
        }
    }
}

setInterval(() => {
    const soundElement = document.getElementById("sounds");
    if (soundElement) {
        const soundsStr = soundElement.textContent;
        const soundsArray = soundsStr.split(",");

        soundsArray.forEach((expirySound) => {
            playUniqueSound(expirySound);
        });
    }
}, 200); // Run every 0.2 seconds
