# BauPlanIO-Webseite (Landingpage + Web-Handbuch) als Container, z. B. für Coolify.
# Build-Kontext ist die Wurzel des Repositorys, weil das Handbuch aus
# docs/handbuch/benutzerhandbuch/ übernommen wird:
#
#   docker build -f web/Dockerfile -t bauplanio-web .
#
# Einstellungen über Umgebungsvariablen, siehe web/README.md.

FROM node:22-bookworm-slim AS abhaengigkeiten
WORKDIR /app/web
# better-sqlite3 bringt fertige Module für linux-x64 und linux-arm64 mit,
# daher ohne Installationsskripte (kein node-gyp, kein Compiler nötig).
COPY web/package.json web/package-lock.json ./
COPY web/vendor ./vendor
RUN npm ci --ignore-scripts

FROM node:22-bookworm-slim AS bau
WORKDIR /app/web
ENV NEXT_TELEMETRY_DISABLED=1
COPY --from=abhaengigkeiten /app/web/node_modules ./node_modules
COPY web/ ./
RUN npm run build

FROM node:22-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0 \
    BAUPLANIO_DATENBANK=/data/auth.sqlite \
    BAUPLANIO_HANDBUCH=/app/handbuch
RUN useradd --system --uid 10001 bauplanio \
    && mkdir -p /data \
    && chown bauplanio /data
COPY --from=bau --chown=bauplanio /app/web/.next/standalone ./
COPY --from=bau --chown=bauplanio /app/web/.next/static ./.next/static
COPY docs/handbuch/benutzerhandbuch ./handbuch
USER bauplanio
EXPOSE 3000
VOLUME /data
HEALTHCHECK --interval=30s --timeout=5s CMD node -e "fetch('http://127.0.0.1:3000/api/auth/ok').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"
CMD ["node", "server.js"]
