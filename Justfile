release:
    git pull
    cargo build --release
    sudo systemctl restart deducers
