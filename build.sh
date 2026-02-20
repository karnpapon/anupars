#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

# TODO: replace with hostname
readonly TARGET_HOST=pi@192.168.1.131
readonly TARGET_PATH=/home/pi/anupars
readonly TARGET_ARCH=aarch64-unknown-linux-gnu # RPi 64-bit OS
readonly SOURCE_PATH=./target/${TARGET_ARCH}/release/anupars

# Check if the target is already installed
if rustup target list --installed | grep -q "$TARGET_ARCH"; then
  echo "Target architecture '$TARGET_ARCH' is already installed."
else
  echo "Target architecture '$TARGET_ARCH' is not installed. Downloading..."
  rustup target add "$TARGET_ARCH"

  if [ $? -eq 0 ]; then
    echo "Successfully installed target architecture '$TARGET_ARCH'."
  else
    echo "Failed to install target architecture '$TARGET_ARCH'. Please check your rustup installation."
  fi
fi

is_docker_installed() {
  command -v docker &>/dev/null
  return $?
}

if ! is_docker_installed; then
  echo "Docker is not installed. Please install Docker Desktop first."
  exit 1
fi

is_docker_running() {
  docker info >/dev/null 2>&1
  return $?
}

if ! is_docker_running; then
  echo "Docker is not running. Starting Docker Desktop..."
  open -a Docker

  while ! is_docker_running; do
    echo "Waiting for Docker to start..."
    sleep 2
  done

  echo "Docker is now running."
else
  echo "Docker is already running."
fi

# Check if `cross` is already installed
if ! command -v cross &>/dev/null; then
  echo "'cross' is not installed. Installing now..."
  rustup update
  cargo install cross
else
  echo "'cross' is already installed."
fi

cross build --release --target=${TARGET_ARCH}
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
