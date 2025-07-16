FROM python:3.14.0b4-bookworm


WORKDIR /app

COPY --chmod=0777 ./target/x86_64-unknown-linux-musl/release/ml-feed-rust .

RUN apt-get update \
    && apt-get install -y ca-certificates

EXPOSE 50051

CMD ["./ml-feed-rust"]