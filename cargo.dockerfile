# Build stage
FROM messense/rust-musl-cross:x86_64-musl AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Fetcher runtime
FROM alpine:3.18.3 AS fetcher
SHELL ["/bin/ash", "-eo", "pipefail", "-c"]
RUN apk add --no-cache chromium ca-certificates tzdata tini
ENV TZ=Europe/Paris
COPY .env /.env
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/fetcher /fetcher
RUN echo "0 7,9,13,17 * * * tini -s -- /fetcher" | crontab -
CMD ["crond", "-f", "-l", "0"]
# CMD ["/fetcher"]

# Rater runtime
FROM alpine:3.18.3 AS rater
RUN apk add --no-cache ca-certificates tzdata
ENV TZ=Europe/Paris
COPY .env /.env
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rater /rater
CMD ["/rater"]
