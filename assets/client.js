const soundCache = new Map();
function playSound(sound) {
    let audio = soundCache.get(sound);
    if (!audio) {
        audio = new Audio(`assets/sounds/${sound}.mp3`);
        soundCache.set(sound, audio);
    }
    audio.currentTime = 0;
    audio.play();
}

document.addEventListener("click", function (event) {
    if (event.target.nodeName === "BUTTON") {
        playSound("button_pressed");
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
        playSound("typing");
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
            const [_, sound] = expirySound.split(";");
            if (sound && !playedSounds.has(expirySound)) {
                playSound(sound);
                playedSounds.add(expirySound);
            }
        });
    }
}, 200);
