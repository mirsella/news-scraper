# Build stage
FROM messense/rust-musl-cross:x86_64-musl AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Fetcher runtime
FROM alpine:3.18.3 AS fetcher
RUN apk add --no-cache chromium ca-certificates tzdata
ENV TZ=Europe/Paris
COPY .env /.env
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/fetcher /fetcher
RUN set -o pipefail && echo "0 */4 * * * /fetcher && pkill -f chromium" | crontab -
CMD ["crond", "-f", "-l", "0"]
# CMD ["/fetcher"]

# Rater runtime
FROM alpine:3.18.3 AS rater
RUN apk add --no-cache ca-certificates tzdata
ENV TZ=Europe/Paris
COPY .env /.env
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rater /rater
CMD ["/rater"]
