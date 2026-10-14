# CoverCraft server for a framework: `covercraft-cli serve` plus the web app.
#
#   docker build -t covercraft .
#   docker run -d -p 7979:7979 -v covercraft-data:/data -e COVERCRAFT_TOKEN=change-me covercraft
#
# The configuration (database connection, preferences; the password in a user-only file)
# lives in /data. Point it at the netmap database from the web app's Settings or with
# `docker exec covercraft covercraft-cli db --kind mysql --host … --user … --password … --database netmap`.

FROM rust:1.97-bookworm AS build
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown \
 && curl -sSL https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xz -C /usr/local/bin trunk
COPY . .
RUN cargo build --release -p covercraft-cli \
 && cd apps/covercraft-web && trunk build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
 && useradd --system --create-home --home-dir /data covercraft
COPY --from=build /src/target/release/covercraft-cli /usr/local/bin/covercraft-cli
COPY --from=build /src/dist/web /app/web
USER covercraft
VOLUME ["/data"]
EXPOSE 7979
ENV COVERCRAFT_HOME=/data
ENV COVERCRAFT_TOKEN=""
CMD ["sh", "-c", "exec covercraft-cli serve --bind 0.0.0.0:7979 --static /app/web --home /data --token \"$COVERCRAFT_TOKEN\" ${COVERCRAFT_LANG:+--lang $COVERCRAFT_LANG}"]
