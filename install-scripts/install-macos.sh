#!/usr/bin/env bash
set -ex

# Install Rust and Cargo
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"

# Make sure Rust has been installed correctly
rustc --version

# Install pyenv
brew install pyenv

# Make sure pyenv has been installed correctly
pyenv -v

# Installing python 3.9 with pyenv
pyenv install 3.9

# Setting python 3.9 as the default local version
pyenv local 3

# Create and enter a virtual environment
python3 -m venv ~/cairo_venv
source ~/cairo_venv/bin/activate

# Install cairo & its dependencies
pip3 install -r requirements.txt

