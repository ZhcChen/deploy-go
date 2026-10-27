ARG BINARIES_IMAGE=scratch
FROM ${BINARIES_IMAGE} AS binaries
FROM python:3.13-alpine3.22
COPY --from=binaries /out/deploy-go-api /usr/local/bin/deploy-go-api
COPY web/ /app/web/
COPY web_server.py /app/web_server.py
COPY initialize.py /app/initialize.py
USER 1000:1000
