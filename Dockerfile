FROM rust:1.67.1-alpine as builder
ARG BINARY=dpsa4fl-janus-tasks
ARG GIT_REVISION=unknown
RUN apk add libc-dev protobuf-dev protoc git
WORKDIR /src
COPY Cargo.toml /src/Cargo.toml
COPY src /src/src
# COPY Cargo.lock /src/Cargo.lock
# COPY aggregator /src/aggregator
# COPY build_script_utils /src/build_script_utils
# COPY client /src/client
# COPY collector /src/collector
# COPY core /src/core
# COPY db /src/db
# COPY integration_tests /src/integration_tests
# COPY interop_binaries /src/interop_binaries
# COPY messages /src/messages
ENV GIT_REVISION ${GIT_REVISION}
ENV CARGO_NET_GIT_FETCH_WITH_CLI true
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/src/target cargo build --release && cp /src/target/release/$BINARY /$BINARY

FROM alpine:3.17.2
ARG BINARY=dpsa4fl-janus-tasks
ARG CONFIG
ARG GIT_REVISION=unknown
LABEL revision ${GIT_REVISION}
RUN echo "config file: $CONFIG"
# COPY --from=builder /src/db/schema.sql /db/schema.sql
COPY --from=builder /$BINARY /$BINARY
# Store the build argument in an environment variable so we can reference it
# from the ENTRYPOINT at runtime.
ENV BINARY=$BINARY
ENV CONFIG=$CONFIG
ENTRYPOINT ["/bin/sh", "-c", "exec /$BINARY --config-file $CONFIG --datastore-keys vWoEFA7F+ojcF+HohGLn/Q"]