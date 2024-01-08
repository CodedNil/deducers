const soundCache = new Map();
function playSound(sound, volume) {
    let audio = soundCache.get(sound);
    if (!audio) {
        audio = new Audio(`assets/sounds/${sound}.mp3`);
        soundCache.set(sound, audio);
    }
    audio.currentTime = 0;
    audio.volume = volume;
    audio.play();
}

document.addEventListener("click", function (event) {
    if (event.target.nodeName === "BUTTON") {
        playSound("button_pressed", 1.0);
        event.target.blur();
    }
});

document.addEventListener("submit", (event) => {
    if (event.target.tagName === "FORM") {
        setTimeout(() => {
            event.target.querySelectorAll('[data-clear-on-submit="true"]').forEach((element) => {
                if (element.tagName === "INPUT") {
                    element.value = "";
                }
            });
        }, 50);
    }
});

document.addEventListener("input", function (event) {
    if (event.target.tagName === "INPUT") {
        playSound("typing", 1.0);
        if (event.target.pattern) {
            const pattern = new RegExp(event.target.pattern);
            if (!pattern.test(event.target.value)) {
                event.target.value = event.target.value.slice(0, -1);
            }
        }
    }
});

const playedSounds = new Set();
setInterval(() => {
    const soundElement = document.getElementById("sounds");
    if (soundElement) {
        soundElement.textContent.split(",").forEach((expirySound) => {
            const [_, sound, volumeString] = expirySound.split(";");
            if (sound && !playedSounds.has(expirySound)) {
                const volume = parseFloat(volumeString);
                playSound(sound, volume);
                playedSounds.add(expirySound);
            }
        });
    }
}, 200);
