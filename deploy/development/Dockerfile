ARG BINARIES_IMAGE
FROM ${BINARIES_IMAGE} AS binaries
FROM alpine:3.22
RUN apk add --no-cache python3 curl ca-certificates && \
    addgroup -g 1000 app && adduser -D -u 1000 -G app app
COPY --from=binaries /out/deploy-go-api /usr/local/bin/deploy-go-api
COPY web/ /app/web/
COPY web_server.py /app/web_server.py
COPY initialize.py /app/initialize.py
USER 1000:1000
