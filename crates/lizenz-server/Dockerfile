# Lizenzserver als Container (z. B. für Coolify). Build-Kontext ist die
# Wurzel des Repositorys:
#
#   docker build -f crates/lizenz-server/Dockerfile -t bauprojektio-lizenz .
#
# Einstellungen über Umgebungsvariablen, siehe crates/lizenz-server/src/main.rs.

FROM rust:1-bookworm AS bau
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p lizenz-server \
    && cp target/release/lizenz-server /usr/local/bin/lizenz-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 lizenz \
    && mkdir -p /data/updates \
    && chown -R lizenz /data
COPY --from=bau /usr/local/bin/lizenz-server /usr/local/bin/lizenz-server
USER lizenz
ENV BAUPROJEKT_UPDATE_DIR=/data/updates \
    BAUPROJEKT_BIND=0.0.0.0:8080
EXPOSE 8080
VOLUME /data/updates
HEALTHCHECK --interval=30s --timeout=5s CMD curl -fsS http://localhost:8080/gesund || exit 1
CMD ["lizenz-server"]
