# to be run with `docker build -v ${PWD}:/app`
# binary is located in `./target/release/`

FROM docker.io/library/debian:stable-slim
LABEL authors="daniel@sigterm.de"

RUN apt-get update && apt-get install -y -y libgtk-3-dev libgtksourceview-3.0-dev cargo
WORKDIR /app
RUN cargo build --release --all-features
