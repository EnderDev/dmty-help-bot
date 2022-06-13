FROM rust:latest

COPY ./ ./

RUN cargo build --release

CMD ["cargo", "run"]